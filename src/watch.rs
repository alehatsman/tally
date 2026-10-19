//! One observer per live slot, turning output bytes into signals.
//!
//! The shape is teleport's, which is proven and one directory over
//! (`docs/04-signals.md`): a reconcile loop rather than a hook on create,
//! because sessions appear when a shell script or another client makes one
//! and nobody tells us.
//!
//! Threads rather than async. The whole job is a handful of blocking reads
//! on a unix socket, one per watched session, and a 2s tick — `tokio`
//! would be a runtime to carry for a loop that sleeps.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::anchor::{Client, Packet, Session};
use crate::signal::Derived;
use crate::slot::Pages;

/// Signals for every session being watched, keyed by session id.
pub type Signals = BTreeMap<String, Derived>;

/// Supervises one watcher per live slot.
///
/// **A poisoned lock here is ignored, not treated as fatal.**
///
/// Both mutexes guard a plain collection — a `BTreeSet<String>` and a
/// [`Signals`] map — and every mutation under them is a single collection
/// call. There is no multi-field invariant a panic could leave torn, so
/// poisoning here carries no information. The `if let Ok` / `is_ok_and`
/// spellings this replaced were not being careful; they were dropping the
/// work and saying nothing.
///
/// What they cost, which the header they replaced got wrong in two places
/// out of three: a `Mutex` stays poisoned for the life of the process, so
/// **none** of these self-healed on the next tick.
///
/// - [`Self::reconcile`] made `fresh` false for every session forever, so
///   no watcher was ever started again. `tallyd` kept running and the grid
///   kept rendering from the anchord list, so nothing looked wrong and
///   nothing was logged.
/// - [`Self::snapshot`] returned an empty map every tick, not one dark
///   frame — every derived signal gone until a restart.
/// - [`Self::retain_known`] skipped the prune every tick, not one, so the
///   map grew for the life of the process.
///
/// Ignoring the poison restores all three to ordinary operation. The panic
/// that set it still killed its own watcher, and the reconcile starts a
/// fresh one.
#[derive(Debug, Clone)]
pub struct Watchers {
    client: Client,
    signals: Arc<Mutex<Signals>>,
    /// Sessions that currently have a thread. A watcher removes itself
    /// when it ends, and the next reconcile starts a fresh one if the
    /// session is still live.
    watching: Arc<Mutex<BTreeSet<String>>>,
}

/// Take one of this module's locks, ignoring poisoning.
///
/// The reasoning, and what the previous spellings cost, is on [`Watchers`].
fn held<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Watchers {
    /// Watch sessions on `client`.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            signals: Arc::new(Mutex::new(Signals::new())),
            watching: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    /// Start a watcher for every live slot that lacks one.
    pub fn reconcile(&self, sessions: &[Session], pages: &Pages, now: i64) {
        for session in crate::model::needs_watcher(sessions, pages) {
            let fresh = held(&self.watching).insert(session.id.clone());
            if fresh {
                self.spawn(&session.id, now);
            }
        }
    }

    /// A copy of the current signals, for rendering.
    pub fn snapshot(&self) -> Signals {
        held(&self.signals).clone()
    }

    /// Forget sessions anchord no longer reports, so the map does not grow
    /// for the life of the process.
    pub fn retain_known(&self, sessions: &[Session]) {
        let known: BTreeSet<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
        held(&self.signals).retain(|id, _| known.contains(id.as_str()));
    }

    fn spawn(&self, id: &str, now: i64) {
        let client = self.client.clone();
        let signals = Arc::clone(&self.signals);
        let watching = Arc::clone(&self.watching);
        let owned = id.to_owned();
        let spawned = std::thread::Builder::new()
            .name(format!("watch-{id}"))
            .spawn(move || {
                run(&client, &owned, &signals, now);
                // Releasing the slot here is what makes a dropped watcher
                // recoverable ([`Watchers::watching`]).
                let _was_there = held(&watching).remove(&owned);
            });
        if let Err(e) = spawned {
            eprintln!("tallyd: could not start a watcher for {id}: {e}");
            let _was_there = held(&self.watching).remove(id);
        }
    }
}

/// Follow one session until it ends or the connection does.
///
/// `now` is the *reconcile's* timestamp, not the thread's, so
/// [`Derived::starting_at`] seeds the output clock from before the attach
/// rather than after it, and a session can cross the idle threshold by
/// that much early. The attach is a unix-socket connect against a 30s
/// threshold, so the error is three orders of magnitude under the thing
/// it perturbs — but it is the reconcile's clock and not the thread's,
/// which is not what the signature suggests.
fn run(client: &Client, id: &str, signals: &Arc<Mutex<Signals>>, now: i64) {
    // From 0 the first time; from `scanned_to` after a drop, so a watcher
    // that fell behind does not re-read the whole log -- worst on exactly
    // the busy sessions that caused the drop.
    let from = held(signals).get(id).map_or(0, |d| d.scanned_to);
    let mut packets = match client.attach(id, from) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("tallyd: attach {id}: {e}");
            return;
        }
    };
    let _existing = held(signals)
        .entry(id.to_owned())
        .or_insert_with(|| Derived::starting_at(now));

    while let Some(packet) = packets.next_packet() {
        match packet {
            Packet::Output { bytes, .. } => {
                let at = crate::now_ms();
                if let Some(d) = held(signals).get_mut(id) {
                    d.feed(&bytes, at);
                }
            }
            // An exit ends the watcher. Whatever was derived while it ran
            // stays; an exited session is rendered from the list, not here.
            Packet::Exit(_) => break,
            Packet::Other => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, state: &str, slot: Option<&str>) -> Session {
        let s = Session::fixture(id, state);
        match slot {
            Some(at) => s.labelled(crate::anchor::LABEL_SLOT, at),
            None => s,
        }
    }

    /// An unreachable anchord must not spin up threads that immediately
    /// die and get restarted every tick.
    #[test]
    fn a_failed_attach_releases_its_slot_for_a_later_retry() {
        let w = Watchers::new(Client::new("/nonexistent/anchor.sock"));
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:1:1"))];

        w.reconcile(&sessions, &pages, 0);
        // The thread fails to attach and removes itself; give it a moment.
        for _ in 0..50 {
            if w.watching.lock().is_ok_and(|x| x.is_empty()) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            w.watching.lock().expect("lock").is_empty(),
            "watcher must release its slot"
        );
    }

    /// Poison both mutexes, then use the `Watchers` normally.
    ///
    /// The regression for the failure [`Watchers`] states the policy for.
    /// Both halves are deterministic — no thread is relied on — and the
    /// signals half is the observable one: before the fix `snapshot`
    /// returned an empty map and this assertion failed.
    #[test]
    fn a_poisoned_lock_does_not_stop_the_watchers() {
        fn poison<T: Send + 'static>(m: &Arc<Mutex<T>>) {
            let m = Arc::clone(m);
            let _expected = std::thread::spawn(move || {
                let _guard = m.lock().expect("fresh mutex");
                panic!("poison it");
            })
            .join();
        }

        let w = Watchers::new(Client::new("/nonexistent/anchor.sock"));
        let _seeded = held(&w.signals).insert("here".to_owned(), Derived::starting_at(0));
        poison(&w.signals);
        poison(&w.watching);
        assert!(w.signals.is_poisoned() && w.watching.is_poisoned());

        // Was: an empty map, every tick, until a restart.
        assert_eq!(
            w.snapshot().into_keys().collect::<Vec<_>>(),
            ["here"],
            "a poisoned lock must not erase the signals"
        );

        // Was: the prune skipped, every tick, until a restart.
        w.retain_known(&[]);
        assert!(w.snapshot().is_empty(), "retain_known must still prune");

        // Was: `fresh` false for every session, so `spawn` was never called
        // and nothing was ever watched again.
        assert!(
            held(&w.watching).insert("a".to_owned()),
            "the watching set must still be usable"
        );
    }

    #[test]
    fn only_live_slots_are_watched() {
        let w = Watchers::new(Client::new("/nonexistent/anchor.sock"));
        let pages = Pages::default();
        let sessions = [
            session("no-slot", "running", None),
            session("exited", "exited", Some("1:1:1")),
            session("bad-slot", "running", Some("nope")),
        ];
        w.reconcile(&sessions, &pages, 0);
        let watching = w.watching.lock().expect("lock");
        assert!(watching.is_empty(), "{watching:?}");
    }

    #[test]
    fn signals_for_vanished_sessions_are_dropped() {
        let w = Watchers::new(Client::new("/nonexistent/anchor.sock"));
        if let Ok(mut s) = w.signals.lock() {
            let _p = s.insert("gone".to_owned(), Derived::starting_at(0));
            let _q = s.insert("here".to_owned(), Derived::starting_at(0));
        }
        w.retain_known(&[session("here", "running", Some("1:1:1"))]);
        let left: Vec<String> = w.snapshot().into_keys().collect();
        assert_eq!(left, ["here"]);
    }
}
