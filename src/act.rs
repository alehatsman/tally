//! Gestures, carried out.
//!
//! The decision of *whether* a gesture applies is
//! [`crate::model::effect`], and it is read twice — once to colour the
//! pad, once here. This module never second-guesses it: if the preview
//! said a pad was a target, the press acts on it, and if it said dim,
//! nothing happens. Two predicates would eventually disagree, and the one
//! that disagrees silently is the one on the destructive button.
//!
//! Everything here talks to anchord and can fail. Failure is reported and
//! dropped: the next reconcile redraws from a fresh `list`, so a kill that
//! did not land shows as a pad that is still green — which is the truth,
//! and better than any error state the surface could invent for it.

use std::collections::BTreeMap;

use crate::anchor::{self, Client, Spawn};
use crate::chord::Action;
use crate::config::{Config, Launch};
use crate::model::{effect, Effect, Pad, Resolved};
use crate::slot::{Pages, Slot};

/// What a gesture did, for the log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// A new session, and its id.
    Spawned(String),
    /// A session ended.
    Killed(String),
    /// Forgotten, and how many rows went with it.
    Purged(usize),
    /// `tl.mute` set (`true`) or cleared.
    Muted(bool),
    /// The gesture did not apply here. The preview said so first.
    Nothing,
}

impl std::fmt::Display for Done {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawned(id) => write!(f, "spawned {id}"),
            Self::Killed(id) => write!(f, "killed {id}"),
            Self::Purged(n) => write!(f, "purged {n}"),
            Self::Muted(true) => write!(f, "muted"),
            Self::Muted(false) => write!(f, "unmuted"),
            Self::Nothing => write!(f, "nothing to do"),
        }
    }
}

/// Everything a press needs in order to act.
#[derive(Debug)]
pub struct Context<'a> {
    /// Where the sessions live.
    pub client: &'a Client,
    /// What the pads can spawn.
    pub config: &'a Config,
    /// What is on the grid right now.
    pub grid: &'a BTreeMap<Slot, Resolved>,
    /// Session ids anchord would forget, by pad.
    pub purgeable: &'a BTreeMap<Slot, Vec<String>>,
}

impl Context<'_> {
    /// The pad as [`crate::model::effect`] sees it. The same construction
    /// the renderer does, which is why they cannot drift.
    fn pad(&self, at: Slot) -> Pad {
        let r = self.grid.get(&at);
        Pad {
            state: r.map(|r| r.state),
            muted: r.is_some_and(|r| r.muted),
            spec: self.launch(at).is_some(),
            purgeable: self.purgeable.get(&at).is_some_and(|ids| !ids.is_empty()),
        }
    }

    /// What to spawn here: the `[[slot]]` entry, or failing that the spec
    /// named by the `tl.spec` label of whatever last ran here.
    ///
    /// The label is the fallback rather than the other way round because
    /// the file is what you edited most recently and meant most
    /// deliberately. The label is how a session spawned from a shell —
    /// `anchor create --label tl.spec=claude` — stays respawnable without
    /// appearing in `slots.toml` at all.
    fn launch(&self, at: Slot) -> Option<&Launch> {
        self.config.launch(at).or_else(|| {
            let spec = self.grid.get(&at)?.spec.as_deref()?;
            self.config.by_spec_name(spec)
        })
    }

    /// A bare tap: spawn a dark pad, or report that it is a jump.
    ///
    /// Returns `Ok(None)` when the pad holds a session — the caller jumps,
    /// because jumping is tmux's business and not anchord's
    /// (`docs/06-jump.md`).
    ///
    /// # Errors
    /// Whatever `create` returned.
    pub fn tap(&self, at: Slot, pages: &Pages) -> Result<Option<Done>, anchor::Error> {
        if self.grid.contains_key(&at) {
            return Ok(None); // a jump; not ours
        }
        let Some(launch) = self.launch(at) else {
            // A dark pad with no spec. Nothing happens, and nothing
            // should: which pads spawn what is your layout, and a press
            // that silently started a shell would be worse than one that
            // did nothing.
            return Ok(Some(Done::Nothing));
        };
        self.spawn(launch, at, pages).map(Some)
    }

    /// A tap under a held modifier.
    ///
    /// # Errors
    /// Whatever anchord returned. A refusal is reported, not retried.
    pub fn chord(&self, action: Action, at: Slot, pages: &Pages) -> Result<Done, anchor::Error> {
        let what = effect(action, self.pad(at));
        if matches!(what, Effect::None | Effect::Skips) {
            return Ok(Done::Nothing);
        }
        match action {
            Action::Kill => self.kill(at),
            Action::Respawn => match self.launch(at) {
                Some(launch) => self.spawn(launch, at, pages),
                None => Ok(Done::Nothing),
            },
            Action::Purge => self.purge(at),
            Action::Mute => self.mute(at, what == Effect::Acts),
        }
    }

    fn spawn(&self, launch: &Launch, at: Slot, pages: &Pages) -> Result<Done, anchor::Error> {
        let Some(address) = at.label(pages) else {
            return Ok(Done::Nothing);
        };
        let mut labels = BTreeMap::new();
        let _slot = labels.insert(anchor::LABEL_SLOT.to_owned(), address);
        let _spec = labels.insert(anchor::LABEL_SPEC.to_owned(), launch.spec.clone());
        self.client
            .create(&Spawn {
                command: &launch.command,
                args: &launch.args,
                cwd: launch.cwd.as_deref(),
                env: &launch.env,
                login_shell: launch.login_shell,
                labels: &labels,
            })
            .map(Done::Spawned)
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
        let Some(r) = self.grid.get(&at) else {
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

    /// Forget every purgeable row on this pad, not just the drawn one.
    ///
    /// A pad with eight clean exits behind it is one gesture to clear,
    /// because eight presses that each look like nothing happening is not
    /// a gesture anybody would complete.
    fn purge(&self, at: Slot) -> Result<Done, anchor::Error> {
        let Some(ids) = self.purgeable.get(&at) else {
            return Ok(Done::Nothing);
        };
        let mut gone = 0;
        for id in ids {
            match self.client.purge(id) {
                Ok(()) => gone += 1,
                // Someone else forgot it between the `list` and the
                // press. That is the outcome we wanted.
                Err(anchor::Error::Refused { ref code, .. }) if code == "not_found" => {}
                // Stop on anything else: if anchord is refusing or gone,
                // the remaining ids will fail the same way, and a burst
                // of identical errors in the log buries the first one.
                Err(e) => return Err(e),
            }
        }
        Ok(Done::Purged(gone))
    }

    fn mute(&self, at: Slot, on: bool) -> Result<Done, anchor::Error> {
        let Some(r) = self.grid.get(&at) else {
            return Ok(Done::Nothing);
        };
        let mut set = BTreeMap::new();
        let remove: &[&str] = if on {
            let _mute = set.insert(anchor::LABEL_MUTE.to_owned(), "1".to_owned());
            &[]
        } else {
            &[anchor::LABEL_MUTE]
        };
        self.client.label(&r.session, &set, remove)?;
        Ok(Done::Muted(on))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::SlotState;

    fn at(row: u8, col: u8) -> Slot {
        Slot { page: 1, row, col }
    }

    fn resolved(slot: Slot, state: SlotState, muted: bool, spec: Option<&str>) -> Resolved {
        Resolved {
            slot,
            session: "01J".to_owned(),
            title: None,
            state,
            muted,
            spec: spec.map(str::to_owned),
        }
    }

    const CONFIG: &str = r#"
[[spec]]
name = "claude"
argv = ["claude"]
[[slot]]
at = "1:1:1"
spec = "claude"
"#;

    fn context<'a>(
        client: &'a Client,
        config: &'a Config,
        grid: &'a BTreeMap<Slot, Resolved>,
        purgeable: &'a BTreeMap<Slot, Vec<String>>,
    ) -> Context<'a> {
        Context {
            client,
            config,
            grid,
            purgeable,
        }
    }

    /// Nothing below talks to anchord: every case here is one the context
    /// settles before a socket would be opened. A socket that is not there
    /// is therefore the right client to test with — and it proves these
    /// paths really do not touch it.
    fn nowhere() -> Client {
        Client::new("/nonexistent/anchor.sock")
    }

    #[test]
    fn a_tap_on_an_occupied_pad_is_not_ours() {
        let mut grid = BTreeMap::new();
        let _r = grid.insert(at(1, 1), resolved(at(1, 1), SlotState::Quiet, false, None));
        let (client, config, purge) = (nowhere(), Config::default(), BTreeMap::new());
        let ctx = context(&client, &config, &grid, &purge);
        assert_eq!(ctx.tap(at(1, 1), &Pages::default()).expect("no call"), None);
    }

    #[test]
    fn a_tap_on_a_dark_pad_with_no_spec_does_nothing() {
        let (client, config, grid, purge) = (
            nowhere(),
            Config::default(),
            BTreeMap::new(),
            BTreeMap::new(),
        );
        let ctx = context(&client, &config, &grid, &purge);
        assert_eq!(
            ctx.tap(at(4, 4), &Pages::default()).expect("no call"),
            Some(Done::Nothing)
        );
    }

    /// The preview and the press read one predicate, so a dim pad is
    /// inert for every gesture — checked here rather than trusted.
    #[test]
    fn a_chord_on_a_pad_the_preview_dimmed_does_nothing() {
        let mut grid = BTreeMap::new();
        // Failed: not killable, not mutable, and respawn needs a spec.
        let _r = grid.insert(
            at(2, 2),
            resolved(at(2, 2), SlotState::Failed { code: 1 }, false, None),
        );
        let (client, config, purge) = (nowhere(), Config::default(), BTreeMap::new());
        let ctx = context(&client, &config, &grid, &purge);
        for action in [Action::Kill, Action::Mute, Action::Respawn] {
            assert_eq!(
                ctx.chord(action, at(2, 2), &Pages::default())
                    .expect("no call"),
                Done::Nothing,
                "{action:?}"
            );
        }
    }

    /// `tl.spec` is the fallback, so a session spawned from a shell stays
    /// respawnable without appearing in `slots.toml`.
    #[test]
    fn a_spec_label_makes_a_pad_respawnable_without_a_slot_entry() {
        let config = Config::parse(CONFIG).expect("valid");
        let mut grid = BTreeMap::new();
        // 5:5 is in no `[[slot]]`, but the dead session there names a spec.
        let _r = grid.insert(
            at(5, 5),
            resolved(
                at(5, 5),
                SlotState::Failed { code: 1 },
                false,
                Some("claude"),
            ),
        );
        let (client, purge) = (nowhere(), BTreeMap::new());
        let ctx = context(&client, &config, &grid, &purge);
        assert_eq!(
            ctx.launch(at(5, 5)).map(|l| l.command.as_str()),
            Some("claude")
        );
        assert!(ctx.pad(at(5, 5)).spec, "the preview must light it too");
    }

    /// The file wins over the label: it is what you edited most recently
    /// and meant most deliberately.
    #[test]
    fn the_slot_table_outranks_a_stale_spec_label() {
        let config = Config::parse(CONFIG).expect("valid");
        let mut grid = BTreeMap::new();
        let _r = grid.insert(
            at(1, 1),
            resolved(
                at(1, 1),
                SlotState::Failed { code: 1 },
                false,
                Some("ghost"),
            ),
        );
        let (client, purge) = (nowhere(), BTreeMap::new());
        let ctx = context(&client, &config, &grid, &purge);
        assert_eq!(
            ctx.launch(at(1, 1)).map(|l| l.spec.as_str()),
            Some("claude")
        );
    }

    /// A dark pad with rows behind it is a purge target, and the pad the
    /// context builds has to say so or the preview will not light it.
    #[test]
    fn a_dark_pad_with_hidden_exits_is_purgeable() {
        let mut purge = BTreeMap::new();
        let _p = purge.insert(at(3, 3), vec!["01A".to_owned(), "01B".to_owned()]);
        let (client, config, grid) = (nowhere(), Config::default(), BTreeMap::new());
        let ctx = context(&client, &config, &grid, &purge);
        let pad = ctx.pad(at(3, 3));
        assert!(pad.purgeable && pad.state.is_none());
        assert_eq!(effect(Action::Purge, pad), Effect::Acts);
    }

    #[test]
    fn a_pad_with_nothing_behind_it_is_not_a_purge_target() {
        let (client, config, grid, purge) = (
            nowhere(),
            Config::default(),
            BTreeMap::new(),
            BTreeMap::new(),
        );
        let ctx = context(&client, &config, &grid, &purge);
        assert_eq!(
            ctx.chord(Action::Purge, at(3, 3), &Pages::default())
                .expect("no call"),
            Done::Nothing
        );
    }

    #[test]
    fn done_says_what_happened() {
        assert_eq!(Done::Muted(true).to_string(), "muted");
        assert_eq!(Done::Muted(false).to_string(), "unmuted");
        assert_eq!(Done::Purged(3).to_string(), "purged 3");
        assert_eq!(Done::Spawned("01J".to_owned()).to_string(), "spawned 01J");
    }
}
