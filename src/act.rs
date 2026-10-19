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

    /// Nothing below talks to anchord: every case here is one the context
    /// settles before a socket would be opened. A socket that is not there
    /// is therefore the right client to test with — and it proves these
    /// paths really do not touch it.
    fn nowhere() -> Client {
        Client::new("/nonexistent/anchor.sock")
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
}
