//! Gestures, carried out.
//!
//! The decision of *whether* a gesture applies is
//! [`crate::effect::effect`], and it is read twice — once to colour the
//! pad, once here. This module never second-guesses it: if the preview
//! said a pad was a target, the press acts on it, and if it said dim,
//! nothing happens. Two predicates would eventually disagree, and the one
//! that disagrees silently is the one on the destructive button.
//!
//! That is a claim about *inputs* as much as about the function, so it is
//! held by construction: the pad [`effect`] is asked about comes from
//! [`crate::model::Pads`], which is also where the renderer gets its
//! pads. There is no second place to build one, because there was, and
//! the two drifted.
//!
//! Everything here talks to anchord and can fail. Failure is reported and
//! dropped: the next reconcile redraws from a fresh `list`, so a kill that
//! did not land shows as a pad that is still green — which is the truth,
//! and better than any error state the surface could invent for it.

use std::collections::BTreeMap;

use crate::anchor::{self, Client};
use crate::chord::Action;
use crate::effect::{effect, Consequence, Effect};
use crate::model::Pads;
use crate::slot::{Pages, Slot};

/// What a gesture did, for the log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// A session ended.
    Killed(String),
    /// Ended rows were forgotten, and how many.
    Forgot(usize),
    /// A pad came into your hand. **Nothing was written** — a pick-up
    /// that is never put down must leave anchord untouched.
    Picked(Slot),
    /// A session changed address ([`crate::chord::Action::Move`]).
    Moved {
        /// The session that moved, so its view can follow
        /// (`docs/06-views.md`).
        session: String,
        /// Where it was.
        from: Slot,
        /// Where it is now.
        to: Slot,
    },
    /// The gesture did not apply here. The preview said so first.
    Nothing,
}

impl std::fmt::Display for Done {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Killed(id) => write!(f, "killed {id}"),
            Self::Forgot(1) => write!(f, "forgot 1 row"),
            Self::Forgot(n) => write!(f, "forgot {n} rows"),
            Self::Picked(at) => write!(f, "picked up {}:{}:{}", at.page, at.row, at.col),
            Self::Moved { from, to, .. } => write!(
                f,
                "moved {}:{}:{} -> {}:{}:{}",
                from.page, from.row, from.col, to.page, to.row, to.col
            ),
            Self::Nothing => write!(f, "nothing to do"),
        }
    }
}

/// Everything a press needs in order to act.
///
/// Two fields and no third: the socket, and the same [`Pads`] the frame
/// was drawn from (module docs).
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// Where the sessions live.
    pub client: &'a Client,
    /// The grid the preview is showing.
    pub pads: Pads<'a>,
}

impl Context<'_> {
    /// A tap under a held modifier.
    ///
    /// # Errors
    /// Whatever anchord returned. A refusal is reported, not retried.
    pub fn chord(&self, action: Action, at: Slot, pages: &Pages) -> Result<Done, anchor::Error> {
        // The **consequence** dispatches, not the button: `Stop` carries
        // two, and which one applies is the question the preview asked a
        // frame ago.
        match effect(action, self.pads.at(at), self.pads.hand()) {
            // `Holds` is the pad already in your hand: putting it back
            // where it came from is not a move and writes nothing.
            // `Occupied` is a live session with your hand already full,
            // and it is not a swap — the colour changed, the answer did
            // not.
            Effect::None | Effect::Skips | Effect::Holds | Effect::Occupied => Ok(Done::Nothing),
            Effect::Acts(Consequence::Ends) => self.kill(at),
            Effect::Acts(Consequence::Forgotten) => Ok(self.forget(at)),
            // A pick-up is surface state, not an anchord call. It is
            // reported rather than performed, and the loop records it.
            Effect::Acts(Consequence::PickedUp) => Ok(Done::Picked(at)),
            Effect::Acts(Consequence::Dropped) => self.put_down(at, pages),
        }
    }

    /// End a session, having first said that we meant to.
    ///
    /// The label goes on **before** the terminate, so there is no window
    /// in which anchord reports a non-zero exit that nothing explains —
    /// the pad would go red for a tick and then dark, on every single
    /// kill.
    ///
    /// If the terminate then fails, the label is taken back off. Leaving
    /// it would be a small lie that outlives the mistake: the session
    /// keeps running, exits badly on its own an hour later, and renders
    /// dark because of a kill that never happened.
    fn kill(&self, at: Slot) -> Result<Done, anchor::Error> {
        let Some(r) = self.pads.grid().get(&at) else {
            return Ok(Done::Nothing);
        };
        let mut set = BTreeMap::new();
        let _killed = set.insert(anchor::LABEL_KILLED.to_owned(), "1".to_owned());
        self.client.label(&r.session, &set, &[])?;
        if let Err(e) = self.client.terminate(&r.session) {
            let undo = self
                .client
                .label(&r.session, &BTreeMap::new(), &[anchor::LABEL_KILLED]);
            if let Err(undo) = undo {
                eprintln!(
                    "tallyd: could not take tl.killed back off {}: {undo}",
                    r.session
                );
            }
            return Err(e);
        }
        Ok(Done::Killed(r.session.clone()))
    }

    /// Put the pad in your hand down here: **one label write.**
    ///
    /// A pad's address is its `tl.slot` and nothing else
    /// ([`crate::chord::Action::Move`]), so no process moves and no pane
    /// is touched *here*. The view follows, but that is tmux's business
    /// and the caller's (`docs/06-views.md`).
    ///
    /// Only the **drawn** session moves. Every ended row at the old
    /// address stays there, so the row it left still knows its directory
    /// and keeps its `>`: dragging a session out of a row is not a claim
    /// that the row never existed.
    ///
    /// **A pad that was drawing a corpse is left with one claimant.**
    /// `tl.slot` is a claim and not a lease, so after the write both
    /// sessions hold the address and [`crate::model::resolve`] draws
    /// whichever anchord lists first — the newest. Move is the only
    /// gesture that places a session anchord *already had*, so it is the
    /// only one that can arrive **older** than what is there: tidy a
    /// morning-old agent onto a pad that failed an hour ago and the
    /// agent is drawn on no pad at all. So the rows underneath are
    /// forgotten, which is the red going away under your finger — the
    /// same thing the hold already does to that pad
    /// ([`Self::forget`]).
    ///
    /// Only for a pad that was **drawing** something. The rows behind a
    /// dark pad are clean exits and kills, which `resolve` skips and
    /// which therefore cannot claim it from anybody; forgetting those
    /// would destroy history to fix nothing.
    ///
    /// The purge goes **after** the label and cannot fail. Before it, a
    /// refused label write would have forgotten those rows for a move
    /// that never happened — and Move must not destroy anything on a
    /// gesture that did not land.
    fn put_down(&self, to: Slot, pages: &Pages) -> Result<Done, anchor::Error> {
        // All three are things `effect` already established before it
        // said `Dropped`; they are re-asked rather than assumed because
        // this function is also the one that writes.
        let (Some(from), Some(address)) = (self.pads.hand(), to.label(pages)) else {
            return Ok(Done::Nothing);
        };
        let Some(r) = self.pads.grid().get(&from) else {
            // It left the grid between the pick-up and the drop: a clean
            // exit. One that ended badly is still drawn, and
            // [`Pads::new`] has already emptied the hand for it.
            return Ok(Done::Nothing);
        };
        let mut set = BTreeMap::new();
        let _slot = set.insert(anchor::LABEL_SLOT.to_owned(), address);
        self.client.label(&r.session, &set, &[])?;
        if self.pads.grid().contains_key(&to) {
            let _cleared = self.forget(to);
        }
        Ok(Done::Moved {
            session: r.session.clone(),
            from,
            to,
        })
    }

    /// Forget every ended row behind a pad.
    ///
    /// **The whole stack, not the newest.** A pad is an address and
    /// anchord holds every session that ever claimed it; forgetting one
    /// of them would leave the pad looking exactly as it did before,
    /// which is the shape of a button you press twice
    /// (`docs/05-interaction.md#stop-does-kill-and-purge`).
    ///
    /// A refusal on one id does not stop the others, and the whole thing
    /// cannot fail — which is why it returns no `Result`. anchord refuses
    /// a row it has already forgotten, and a `purge` that gave up halfway
    /// would leave the pad in a state no further press could clear.
    fn forget(&self, at: Slot) -> Done {
        let mut forgotten = 0;
        for id in self.pads.rows(at) {
            match self.client.purge(id) {
                Ok(()) => forgotten += 1,
                Err(e) => eprintln!("tallyd: could not forget {id}: {e}"),
            }
        }
        if forgotten == 0 {
            // Rows were there and anchord kept all of them. Saying
            // "forgot 0 rows" would read as a thing that happened.
            return Done::Nothing;
        }
        Done::Forgot(forgotten)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::model::{frame_from, Resolved, View};
    use crate::render;
    use crate::signal::SlotState;
    use crate::surface::Button;

    fn at(row: u8, col: u8) -> Slot {
        Slot { page: 1, row, col }
    }

    fn resolved(slot: Slot, state: SlotState, muted: bool) -> Resolved {
        Resolved {
            slot,
            session: "01J".to_owned(),
            title: None,
            state,
            muted,
            claude: None,
            cwd: None,
        }
    }

    /// The view both frame tests below draw from: the corner held, and
    /// everything else at rest. They assert different things and keep
    /// doing so — only the twelve fields are shared, so the next field
    /// added to [`View`] lands in one place and not two.
    fn held_view<'a>(pages: &'a Pages, pressed: &'a BTreeSet<Action>) -> View<'a> {
        View {
            pages,
            offset: 0,
            anchord_reachable: true,
            now: 0,
            can_go_back: false,
            queued: 0,
            launchable: &NOTHING_LAUNCHES,
            held: Some(Action::Stop),
            hand: None,
            pressed,
            art: false,
            mode: Action::Stop,
        }
    }

    /// No `>` lights in either test: neither builds a session list.
    static NOTHING_LAUNCHES: BTreeSet<u8> = BTreeSet::new();

    fn context<'a>(
        client: &'a Client,
        grid: &'a BTreeMap<Slot, Resolved>,
        purgeable: &'a BTreeMap<Slot, Vec<String>>,
    ) -> Context<'a> {
        Context {
            client,
            pads: Pads::new(grid, purgeable, None),
        }
    }

    /// Nothing handed this talks to anchord: every case it is used for is
    /// one the context settles before a socket would be opened. A socket
    /// that is not there is therefore the right client to test with — and
    /// it proves those paths really do not touch it. The one test about
    /// what gets *written* uses [`recording`] instead.
    fn nowhere() -> Client {
        Client::new("/nonexistent/anchor.sock")
    }

    /// A fake anchord that says yes to everything and keeps what it was
    /// asked.
    ///
    /// It stops at `stop_after` requests or at a deadline, whichever
    /// comes first. [`Client`] opens a connection per op, so a test that
    /// allows more calls than it expects asserts "and nothing else" by
    /// waiting the deadline out — where accepting forever would hang the
    /// suite on exactly the regression it is there to catch.
    fn recording(name: &str, stop_after: usize) -> (Client, JoinHandle<Vec<String>>) {
        let dir = std::env::temp_dir().join("tally-act-test");
        std::fs::create_dir_all(&dir).expect("a temp dir for the socket");
        let path = dir.join(format!("{name}-{}.sock", std::process::id()));
        // A socket file outlives the process that bound it, so a crashed
        // earlier run would make `bind` fail with "address in use".
        let _stale = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("bind");
        listener
            .set_nonblocking(true)
            .expect("a listener that can be given up on");
        let server = std::thread::spawn(move || {
            let mut seen: Vec<String> = Vec::new();
            let deadline = Instant::now() + Duration::from_millis(750);
            while seen.len() < stop_after && Instant::now() < deadline {
                // Nothing waiting, or a real error: either way the
                // deadline is what ends this, and a client that never
                // called is the assertion the test makes.
                let Ok((mut stream, _addr)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                stream
                    .set_nonblocking(false)
                    .expect("a stream that can be read");
                let mut request = String::new();
                let _read = BufReader::new(stream.try_clone().expect("clone"))
                    .read_line(&mut request)
                    .expect("read the request");
                stream
                    .write_all(b"{\"id\":1,\"ok\":true}\n")
                    .expect("write the reply");
                seen.push(request);
            }
            seen
        });
        (Client::new(&path), server)
    }

    /// The preview and the press read one predicate, so a dim pad is
    /// inert for every gesture — checked here rather than trusted.
    #[test]
    fn a_chord_on_a_pad_the_preview_dimmed_does_nothing() {
        let mut grid = BTreeMap::new();
        // Failed: not killable, not mutable, and respawn needs a spec.
        let _r = grid.insert(
            at(2, 2),
            resolved(at(2, 2), SlotState::Failed { code: 1 }, false),
        );
        let (client, purge) = (nowhere(), BTreeMap::new());
        let ctx = context(&client, &grid, &purge);
        assert_eq!(
            ctx.chord(Action::Stop, at(2, 2), &Pages::default())
                .expect("no call"),
            Done::Nothing
        );
    }

    /// The pad the merged button exists for, and the one that makes it
    /// risky: a clean exit keeps its `tl.slot` and draws as **nothing**,
    /// so this pad is dark on every frame until the hold reveals it
    /// (`docs/05-interaction.md#stop-does-kill-and-purge`).
    ///
    /// Asserted as all three halves agreeing — the pad is dark at rest,
    /// dark red under the hold, and the press is handed every id, not
    /// the newest.
    #[test]
    fn a_dark_pad_with_history_is_dark_until_the_hold_reveals_it() {
        let grid = BTreeMap::new(); // nothing drawn here, ever
        let mut purge = BTreeMap::new();
        let _p = purge.insert(
            at(5, 5),
            vec!["01J_c".to_owned(), "01J_b".to_owned(), "01J_a".to_owned()],
        );
        let client = nowhere();
        let ctx = context(&client, &grid, &purge);
        let pressed = BTreeSet::new();

        assert!(
            ctx.pads.at(at(5, 5)).state.is_none(),
            "dark at rest: there is nothing to draw"
        );
        assert_eq!(
            effect(Action::Stop, ctx.pads.at(at(5, 5)), None),
            Effect::Acts(Consequence::Forgotten),
            "and a target under the hold"
        );

        let frame = frame_from(&ctx.pads, &held_view(&Pages::default(), &pressed));
        assert_eq!(
            frame.get(Button::Grid { row: 5, col: 5 }),
            render::preview(Effect::Acts(Consequence::Forgotten)),
            "the hold is the only thing that has ever shown these rows"
        );
        assert_eq!(
            ctx.pads.rows(at(5, 5)).len(),
            3,
            "and the press is handed the whole stack, not the newest"
        );
    }

    /// The regression the shared [`Pads`] exists to make unrepresentable.
    ///
    /// It was found as a `tl.spec` that no longer resolved: the renderer
    /// counted the label alone and the press resolved it, so holding
    /// respawn painted the pad green — *this will respawn* — and the press
    /// did nothing. Both the label and the respawn chord are gone; the
    /// invariant they cost is what this still holds.
    ///
    /// Asserted as the two halves agreeing rather than as one expected
    /// colour: what matters is that the pixel and the press tell the same
    /// story, whichever story that is.
    ///
    /// It reaches the dim arm because the purge map is **empty** — a
    /// dead pad anchord would forget is a target now
    /// (`docs/05-interaction.md`), and that case is the test below.
    #[test]
    fn the_preview_and_the_press_agree_about_a_dead_pad() {
        let mut grid = BTreeMap::new();
        let _r = grid.insert(
            at(5, 5),
            resolved(at(5, 5), SlotState::Failed { code: 1 }, false),
        );
        let (client, purge, pages) = (nowhere(), BTreeMap::new(), Pages::default());
        let ctx = context(&client, &grid, &purge);
        let pressed = BTreeSet::new();
        let frame = frame_from(&ctx.pads, &held_view(&pages, &pressed));

        assert_eq!(
            frame.get(Button::Grid { row: 5, col: 5 }),
            render::preview(Effect::Skips),
            "a pad that already ended is not a kill target"
        );
        assert_eq!(
            ctx.chord(Action::Stop, at(5, 5), &Pages::default())
                .expect("no call"),
            Done::Nothing,
            "and the press must agree with the pixel"
        );
    }

    /// A session can die while it is in your hand: the pick-up refuses a
    /// dead pad, so this is one that ended afterwards with the modifier
    /// still down. A failed or lost row stays in the grid, so `put_down`
    /// found it and rewrote the address of a session that had ended —
    /// the one thing Move must never do. Asserted as the pixel and the
    /// press agreeing, and the press getting as far as a socket would
    /// show: `nowhere` has none, so an `Ok` is a write that never began.
    #[test]
    fn a_session_that_died_in_your_hand_cannot_be_dropped() {
        let held = at(2, 2);
        for dead in [
            SlotState::Failed { code: 1 },
            SlotState::Lost { reclaimable: false },
        ] {
            let mut grid = BTreeMap::new();
            let _r = grid.insert(held, resolved(held, dead, false));
            let (client, purge, pages) = (nowhere(), BTreeMap::new(), Pages::default());
            let ctx = Context {
                client: &client,
                pads: Pads::new(&grid, &purge, Some(held)),
            };

            assert_eq!(
                effect(Action::Move, ctx.pads.at(at(5, 5)), ctx.pads.hand()),
                Effect::None,
                "{dead:?}: an empty pad is not a landing for a session that ended"
            );
            assert_eq!(
                ctx.chord(Action::Move, at(5, 5), &pages).expect("no call"),
                Done::Nothing,
                "{dead:?}: and the press must agree with the pixel"
            );
        }
    }

    /// A drop onto a pad that is drawing a corpse must leave that pad
    /// with **one** claimant.
    ///
    /// `tl.slot` is a claim and not a lease, so both sessions hold the
    /// address the moment the label lands, and `resolve` draws whichever
    /// anchord lists first — the newest. Every other way a session
    /// reaches a contested pad creates it on arrival and so is always the
    /// newest; Move places a session anchord already had, and an agent
    /// running since the morning tidied onto a pad that failed an hour
    /// ago is older than the corpse. It would be drawn on no pad at all:
    /// not jumpable, not in `tally ls`, and nothing on the surface saying
    /// why.
    ///
    /// Asserted against a fake anchord, because the claim is about the
    /// calls and nothing else can see them. The dark pad is the other
    /// half: its rows are ones `resolve` already skips, and a drop that
    /// forgot them would be destroying history to fix nothing.
    #[test]
    fn a_drop_forgets_the_corpse_it_landed_on() {
        let (held, dead, dark) = (at(2, 2), at(5, 5), at(6, 6));
        let pages = Pages::default();
        let mut grid = BTreeMap::new();
        let _h = grid.insert(held, resolved(held, SlotState::Working, false));
        let _d = grid.insert(
            dead,
            Resolved {
                session: "01J_dead".to_owned(),
                ..resolved(dead, SlotState::Failed { code: 1 }, false)
            },
        );
        let mut purge = BTreeMap::new();
        let _onto = purge.insert(dead, vec!["01J_dead".to_owned(), "01J_older".to_owned()]);
        let _under = purge.insert(dark, vec!["01J_gone".to_owned()]);

        for (to, allowed, want) in [(dead, 4, 3), (dark, 3, 1)] {
            let (client, server) = recording(&format!("drop-{}", to.row), allowed);
            let ctx = Context {
                client: &client,
                pads: Pads::new(&grid, &purge, Some(held)),
            };
            assert_eq!(
                effect(Action::Move, ctx.pads.at(to), ctx.pads.hand()),
                Effect::Acts(Consequence::Dropped),
                "the preview promised a landing on {to:?}"
            );
            assert_eq!(
                ctx.chord(Action::Move, to, &pages)
                    .expect("the address write"),
                Done::Moved {
                    session: "01J".to_owned(),
                    from: held,
                    to,
                },
            );
            let seen = server.join().expect("server thread");
            let _gone = std::fs::remove_file(client.path());
            let ops: Vec<serde_json::Value> = seen
                .iter()
                .map(|line| serde_json::from_str(line).expect("anchord was sent JSON"))
                .collect();
            assert_eq!(ops.len(), want, "{to:?}: {ops:?}");
            assert_eq!(ops[0]["op"], "label", "the address first, always");
            assert_eq!(ops[0]["session"], "01J");
            assert_eq!(
                ops[0]["set"][anchor::LABEL_SLOT],
                to.label(&pages).expect("an address for a declared page")
            );
            for (op, id) in ops.iter().skip(1).zip(["01J_dead", "01J_older"]) {
                assert_eq!(op["op"], "purge");
                assert_eq!(op["session"], id, "the whole stack, newest first");
            }
        }
    }
}
