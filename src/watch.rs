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

use crate::anchor::{Client, Frame, Session};
use crate::signal::Derived;
use crate::slot::Pages;

/// Signals for every session being watched, keyed by session id.
pub type Signals = BTreeMap<String, Derived>;

/// Supervises one watcher per live slot.
#[derive(Debug, Clone)]
pub struct Watchers {
    client: Client,
    signals: Arc<Mutex<Signals>>,
    /// Sessions that currently have a thread. A watcher removes itself
    /// when it ends, and the next reconcile starts a fresh one if the
    /// session is still live.
    watching: Arc<Mutex<BTreeSet<String>>>,
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
            let fresh = self
                .watching
                .lock()
                .is_ok_and(|mut w| w.insert(session.id.clone()));
            if fresh {
                self.spawn(&session.id, now);
            }
        }
    }

    /// A copy of the current signals, for rendering.
    pub fn snapshot(&self) -> Signals {
        self.signals.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Forget sessions anchord no longer reports, so the map does not grow
    /// for the life of the process.
    pub fn retain_known(&self, sessions: &[Session]) {
        let known: BTreeSet<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
        if let Ok(mut signals) = self.signals.lock() {
            signals.retain(|id, _| known.contains(id.as_str()));
        }
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
                // recoverable: the next reconcile sees no thread for a
                // still-live session and starts a fresh one.
                if let Ok(mut w) = watching.lock() {
                    let _was_there = w.remove(&owned);
                }
            });
        if let Err(e) = spawned {
            eprintln!("tallyd: could not start a watcher for {id}: {e}");
            if let Ok(mut w) = self.watching.lock() {
                let _was_there = w.remove(id);
            }
        }
    }
}

/// Follow one session until it ends or the connection does.
fn run(client: &Client, id: &str, signals: &Arc<Mutex<Signals>>, now: i64) {
    // From 0 the first time; from `scanned_to` after a drop, so a watcher
    // that fell behind does not re-read the whole log -- worst on exactly
    // the busy sessions that caused the drop.
    let from = signals
        .lock()
        .map_or(0, |s| s.get(id).map_or(0, |d| d.scanned_to));
    let mut frames = match client.attach(id, from) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("tallyd: attach {id}: {e}");
            return;
        }
    };
    if let Ok(mut s) = signals.lock() {
        let _existing = s
            .entry(id.to_owned())
            .or_insert_with(|| Derived::starting_at(now));
    }

    while let Some(frame) = frames.next_frame() {
        match frame {
            Frame::Output { bytes, .. } => {
                let at = crate::now_ms();
                if let Ok(mut s) = signals.lock() {
                    if let Some(d) = s.get_mut(id) {
                        d.feed(&bytes, at);
                    }
                }
            }
            // An exit ends the watcher. Whatever was derived while it ran
            // stays; an exited session is rendered from the list, not here.
            Frame::Exit(_) => break,
            Frame::Other => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, state: &str, slot: Option<&str>) -> Session {
        let mut labels = BTreeMap::new();
        if let Some(s) = slot {
            let _previous = labels.insert(crate::anchor::LABEL_SLOT.to_owned(), s.to_owned());
        }
        Session {
            id: id.to_owned(),
            state: state.to_owned(),
            exit_code: None,
            lost_reason: None,
            labels,
            output_bytes: 0,
            pid: None,
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
        assert!(w.watching.lock().expect("lock").is_empty());
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
