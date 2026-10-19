//! Deriving "what is this agent doing" from its output bytes.
//!
//! anchor reports `running` / `closing` / `exited` / `lost` and nothing
//! about what the process inside is up to — that is a policy about what a
//! UI shows, so it lives here (`docs/04-signals.md`).
//!
//! Nothing in this module is allowed to become load-bearing enough that
//! losing it breaks the surface. Lose the bell, lose the amber; the pad is
//! still green and still jumps.

/// How long a session must produce nothing before it counts as idle.
pub const IDLE_THRESHOLD_MS: i64 = 30_000;
/// How long after a bell the silence must last before the pad goes amber.
///
/// Deliberately **not** [`IDLE_THRESHOLD_MS`]. Attention wants to fire
/// fast, and "nothing since the bell" is a far cheaper claim than "idle".
pub const ANSWER_QUIET_MS: i64 = 2_000;

/// Where a `NeedsAnswer` came from. Carrying the distinction from the start
/// means accepting an authoritative signal later is a match arm rather than
/// a rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Scraped from the byte stream.
    Derived,
    /// A hook said so, via the `tl.attention` label.
    Declared,
}

/// What a pad renders.
///
/// **The grid shows what is alive or what is wrong.** Everything else is
/// dark — including a session that finished cleanly, which needs nothing
/// from anyone and whose slot is free again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// Nothing here, or nothing left to do.
    Empty,
    /// Alive and producing output.
    Working,
    /// Alive and silent. The most common state an interactive agent is in:
    /// it finished its turn and is sitting at a prompt, which is why this
    /// is green rather than something that looks finished.
    Quiet,
    /// It rang and went silent, or a hook said so outright.
    NeedsAnswer {
        /// Scraped from the stream, or declared by a hook.
        source: Source,
    },
    /// Exited non-zero. Stays visible because you need to go and look.
    Failed {
        /// The exit status anchor recorded.
        code: i32,
    },
    /// anchor cannot say how this ended. `EXITED` means the daemon can
    /// account for it, `LOST` means it cannot (`anchor/docs/02-model.md`):
    /// either the PTY died with the daemon, or a kill was never observed
    /// completing — in which case the process may still be out there.
    ///
    /// Its own variant because `lost` arrives in **bulk**. anchord restarts
    /// and every running session becomes a question mark at once; a grid of
    /// red would say "twenty things failed", which is false and sends you
    /// debugging instead of purging.
    Lost {
        /// Is the process definitively gone, so the pad can be claimed
        /// again?
        ///
        /// True only for `lost_reason=daemon_restart`. `lost` usually
        /// means *anchor cannot account for this*, which includes "it may
        /// still be running" — and spawning a second agent into the same
        /// repo with one pad to watch both is worse than a refusal. A
        /// daemon restart is the one reason that cannot mean that, because
        /// anchord owns the PTYs and took them with it.
        ///
        /// **It does not change the colour.** The row is still lost and
        /// the pad is still dark red: the daemon still cannot account for
        /// what that session did. What it changes is whether the pad is
        /// free to claim ([#9](https://github.com/alehatsman/tally/issues/9)).
        reclaimable: bool,
    },
}

impl SlotState {
    /// Is this pad free for something new?
    ///
    /// Dark, failed, or held only by a row anchord lost to its own
    /// restart. **One predicate, and it lives on the state rather than on
    /// a pad** because the two things that ask are a grid and a shell, and
    /// only one of them has a [`crate::effect::Pad`] to ask with. They did
    /// not share one before, and a grid that disagrees with the CLI about
    /// what a pad is, is a grid that lies about what pressing it does
    /// ([#9](https://github.com/alehatsman/tally/issues/9)).
    #[must_use]
    pub fn frees_the_pad(self) -> bool {
        matches!(
            self,
            Self::Empty | Self::Failed { .. } | Self::Lost { reclaimable: true }
        )
    }
}

/// anchor's view of a session's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Alive, with a live PTY.
    Running,
    /// Termination requested; the child has not been reaped yet. Still
    /// **alive**, and it has no exit code to render.
    Closing,
    /// The child is gone and anchor can say how it ended.
    Exited,
    /// anchor cannot account for how it ended.
    Lost,
}

/// Signals scraped from one session's output stream.
#[derive(Debug, Default, Clone)]
pub struct Derived {
    /// When a BEL was last seen. `None` if never.
    ///
    /// **Recorded, not consulted.** No predicate reads it, and the one
    /// that looks like it should must not: `docs/04-signals.md` writes
    /// "bell since the last output" as a timestamp comparison, and the
    /// BEL arrives *inside* an output chunk, so this and `last_output_ms`
    /// are set to the same instant and `>=` versus `>` flips the answer.
    /// The outstanding-bell flag is a flag for exactly that reason. Kept
    /// because it is the only record of *when* a session last rang, and
    /// the spec's mechanism sketch names it.
    pub last_bell_ms: Option<i64>,
    /// When output last arrived.
    pub last_output_ms: i64,
    /// One past the last byte scanned, so a watcher restarted after being
    /// dropped for backpressure does not re-read the whole log.
    pub scanned_to: u64,
    /// A bell is outstanding: it rang and has not spoken since.
    bell_pending: bool,
    /// What the agent last called itself, if it said.
    title: Option<String>,
    /// Chunk-boundary-safe OSC state. Also the only thing that can tell a
    /// real bell from a title's terminator.
    scanner: crate::osc::Scanner,
}

impl Derived {
    /// A watcher that has just attached and seen nothing yet.
    ///
    /// `now` seeds the output clock, so a session does not read as idle
    /// for the first 30 seconds purely because its watcher is young.
    pub fn starting_at(now: i64) -> Self {
        Self {
            last_output_ms: now,
            ..Self::default()
        }
    }

    /// Fold one chunk of output in.
    ///
    /// The subtlety that makes this a flag rather than a timestamp
    /// comparison: **the BEL arrives inside an output chunk**, so the chunk
    /// carrying it sets the bell time and the output time to the same
    /// instant, and `>=` versus `>` on two equal numbers flips the answer.
    ///
    /// `tail` is computed before the bell branch so that one rule —
    /// *non-whitespace output cancels a pending bell* — covers both the
    /// bell's own chunk and every chunk after it.
    pub fn feed(&mut self, bytes: &[u8], now: i64) {
        if bytes.is_empty() {
            return;
        }
        // The scan runs first, because **`BEL` is also an OSC string
        // terminator**. `ESC ] 0 ; title BEL` contains a `0x07` that is not
        // a ring, and an agent that sets its terminal title would otherwise
        // look like one that asked a question — a pad flashing amber for
        // nothing, which is the failure M1 is counting.
        let scan = self.scanner.feed(bytes);
        if let Some(title) = scan.titles.into_iter().next_back() {
            self.title = Some(title);
        }
        let tail = match scan.last_bell {
            Some(i) => {
                self.last_bell_ms = Some(now);
                self.bell_pending = true;
                bytes.get(i + 1..).unwrap_or_default()
            }
            None => bytes,
        };
        // Whitespace never cancels: a bell is almost always followed by a
        // newline before the prompt, and which chunk that lands in is a
        // buffering accident nobody should be able to observe.
        if !tail.iter().all(u8::is_ascii_whitespace) {
            self.bell_pending = false;
        }
        self.last_output_ms = now;
        self.scanned_to += bytes.len() as u64;
    }

    /// Bell outstanding, and quiet for long enough to believe it.
    pub fn needs_answer(&self, now: i64) -> bool {
        self.bell_pending && now - self.last_output_ms > ANSWER_QUIET_MS
    }

    /// What the agent last called itself.
    ///
    /// Allowed to be absent, always. Losing the title loses a word, never
    /// a pad: no signal here may become load-bearing enough that losing it
    /// breaks the surface.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Nothing for [`IDLE_THRESHOLD_MS`].
    pub fn idle(&self, now: i64) -> bool {
        now - self.last_output_ms >= IDLE_THRESHOLD_MS
    }
}

/// Everything about a session that the state decision needs.
#[derive(Debug, Clone, Copy)]
pub struct Session<'a> {
    /// Where anchor says this session is in its lifecycle.
    pub state: SessionState,
    /// Set once the session reaches a terminal state anchor can account for.
    pub exit_code: Option<i32>,
    /// `tl.mute` — suppresses attention, nothing else.
    pub muted: bool,
    /// `tl.attention` — a hook declared this session is waiting.
    pub declared_attention: bool,
    /// `None` when no watcher has produced anything yet.
    pub derived: Option<&'a Derived>,
    /// `lost_reason=daemon_restart` — the one reason `lost` means gone.
    pub lost_to_restart: bool,
}

/// The ordered match that is the whole opinion (`docs/04-signals.md`).
///
/// `session` is `None` when nothing holds the slot. Whether a spec *could*
/// go there is deliberately not an input: an empty pad looks the same
/// either way, because which pads spawn what is your layout, held in
/// muscle memory rather than read off the grid.
pub fn slot_state(session: Option<Session<'_>>, now: i64) -> SlotState {
    let Some(s) = session else {
        return SlotState::Empty;
    };

    match s.state {
        // Terminal states outrank attention: an agent that rang and then
        // died has nothing left to answer.
        SessionState::Lost => {
            return SlotState::Lost {
                reclaimable: s.lost_to_restart,
            }
        }
        // A clean exit is not shown at all. It needs nothing and its slot
        // is free again, so it stops being tally's business -- the row
        // stays in anchor for `anchor logs`, and the pad goes dark.
        SessionState::Exited => {
            return match s.exit_code {
                Some(0) => SlotState::Empty,
                Some(code) => SlotState::Failed { code },
                // A terminal row with no code is precisely anchor saying it
                // cannot account for the ending, which is what Lost means.
                // Not reclaimable: anchord is up and still cannot say, so
                // this is the case where something may be out there.
                None => SlotState::Lost { reclaimable: false },
            };
        }
        // Closing is still alive and has no exit code yet, so it falls
        // through to the live states below.
        SessionState::Running | SessionState::Closing => {}
    }

    if !s.muted {
        if s.declared_attention {
            return SlotState::NeedsAnswer {
                source: Source::Declared,
            };
        }
        if s.derived.is_some_and(|d| d.needs_answer(now)) {
            return SlotState::NeedsAnswer {
                source: Source::Derived,
            };
        }
    }

    // A session with no watcher yet, or one that has gone quiet, is alive
    // and idle -- green and still, not a state of its own.
    match s.derived {
        Some(d) if !d.idle(now) => SlotState::Working,
        _ => SlotState::Quiet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_760_000_000_000;

    fn fed(chunks: &[(&[u8], i64)]) -> Derived {
        let mut d = Derived {
            last_output_ms: T0,
            ..Derived::default()
        };
        for (bytes, at) in chunks {
            d.feed(bytes, *at);
        }
        d
    }

    fn live(derived: &Derived) -> Session<'_> {
        Session {
            lost_to_restart: false,
            state: SessionState::Running,
            exit_code: None,
            muted: false,
            declared_attention: false,
            derived: Some(derived),
        }
    }

    #[test]
    fn bell_then_silence_asks_for_an_answer() {
        let d = fed(&[(b"done\x07", T0)]);
        assert!(!d.needs_answer(T0 + 1_000), "2s has not passed");
        assert!(d.needs_answer(T0 + 3_000));
    }

    /// It rang and kept going. Without this the pad goes amber 2s after
    /// every bell, forever, which is the easiest way to make the signal
    /// untrustworthy.
    #[test]
    fn a_bell_with_trailing_output_in_the_same_chunk_does_not() {
        let d = fed(&[(b"\x07still working on it", T0)]);
        assert!(!d.needs_answer(T0 + 10_000));
    }

    #[test]
    fn a_bell_then_output_in_a_later_chunk_does_not() {
        let d = fed(&[(b"\x07", T0), (b"still working", T0 + 100)]);
        assert!(!d.needs_answer(T0 + 10_000));
    }

    /// The mirror image. A bell is almost always followed by a newline
    /// before the prompt; treat that as "it kept going" and the signal
    /// never fires at all. Which chunk it lands in is a buffering accident.
    #[test]
    fn whitespace_after_a_bell_never_cancels_it() {
        let same_chunk = fed(&[(b"ready?\x07\r\n", T0)]);
        assert!(
            same_chunk.needs_answer(T0 + 3_000),
            "trailing newline cancelled it"
        );

        let next_chunk = fed(&[(b"ready?\x07", T0), (b"\r\n", T0 + 50)]);
        assert!(
            next_chunk.needs_answer(T0 + 3_000),
            "newline in the next chunk cancelled it"
        );

        let spaces = fed(&[(b"\x07", T0), (b"   \t\n", T0 + 10)]);
        assert!(spaces.needs_answer(T0 + 3_000));
    }

    /// The defect this scanner was ported to fix. `ESC ] 0 ; title BEL`
    /// ends in a `0x07` that is a string terminator, not a ring — and
    /// Claude Code sets its title constantly. A naive `bytes.contains(7)`
    /// turns every title into a question and floods the grid with amber.
    #[test]
    fn setting_a_terminal_title_is_not_asking_a_question() {
        let d = fed(&[(b"\x1b]0;\xe2\x9c\xb3 tally \xe2\x80\x94 building\x07", T0)]);
        assert!(!d.needs_answer(T0 + 10_000), "a title must not go amber");
        assert_eq!(d.title(), Some("✳ tally — building"));
    }

    /// And the title must not mask a real question that follows it.
    #[test]
    fn a_title_then_a_real_bell_still_asks() {
        let d = fed(&[(b"\x1b]0;tally\x07", T0), (b"what next?\x07", T0 + 100)]);
        assert!(d.needs_answer(T0 + 3_000));
        assert_eq!(d.title(), Some("tally"));
    }

    /// A title arriving *after* a bell is output, so it cancels — the agent
    /// spoke again. Whitespace does not cancel, but an escape sequence is
    /// not whitespace.
    #[test]
    fn a_title_after_a_bell_cancels_it() {
        let d = fed(&[(b"done?\x07", T0), (b"\x1b]0;working\x07", T0 + 100)]);
        assert!(!d.needs_answer(T0 + 10_000), "it started talking again");
    }

    #[test]
    fn answering_clears_the_amber() {
        let mut d = fed(&[(b"?\x07", T0)]);
        assert!(d.needs_answer(T0 + 3_000));
        d.feed(b"thinking...", T0 + 4_000);
        assert!(!d.needs_answer(T0 + 10_000));
    }

    /// It is a flag, not a timer: a pad that has been waiting ten minutes
    /// should still be amber.
    #[test]
    fn attention_does_not_time_out() {
        let d = fed(&[(b"?\x07", T0)]);
        assert!(d.needs_answer(T0 + 600_000));
    }

    #[test]
    fn a_lot_of_output_with_no_bell_is_just_working() {
        let mut d = fed(&[(b"boot", T0)]);
        let chunk = vec![b'x'; 64 * 1024];
        for i in 0..160 {
            d.feed(&chunk, T0 + i);
        }
        assert!(!d.needs_answer(T0 + 100_000));
        assert_eq!(d.scanned_to, 4 + 160 * 64 * 1024);
        // Nothing here grows with the stream. The only heap `Derived` owns
        // is the scanner's mid-sequence buffer and the last title, and
        // plain output puts nothing in either -- so 10MB costs no memory.
        assert_eq!(d.scanner.buffered(), 0, "ordinary output retained bytes");
        assert_eq!(d.title(), None);
    }

    #[test]
    fn scanned_to_advances_so_a_restarted_watcher_resumes() {
        let d = fed(&[(b"abc", T0), (b"defg", T0 + 1)]);
        assert_eq!(d.scanned_to, 7);
    }

    // --- precedence ---------------------------------------------------

    #[test]
    fn an_empty_slot_is_empty_whatever_the_config_says() {
        assert_eq!(slot_state(None, T0), SlotState::Empty);
    }

    /// Terminal states outrank attention, and a clean exit leaves no mark:
    /// an agent that rang and then finished wants nothing.
    #[test]
    fn a_bell_then_a_clean_exit_goes_dark_not_amber() {
        let d = fed(&[(b"?\x07", T0)]);
        let s = Session {
            state: SessionState::Exited,
            exit_code: Some(0),
            ..live(&d)
        };
        assert_eq!(slot_state(Some(s), T0 + 5_000), SlotState::Empty);
    }

    #[test]
    fn a_bell_then_a_failure_is_red_not_amber() {
        let d = fed(&[(b"?\x07", T0)]);
        let s = Session {
            state: SessionState::Exited,
            exit_code: Some(2),
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(s), T0 + 5_000),
            SlotState::Failed { code: 2 }
        );
    }

    /// A terminal row with no exit code is anchor saying it cannot account
    /// for the ending -- which is exactly Lost, not a zero.
    #[test]
    fn a_terminal_row_without_a_code_is_lost_not_clean() {
        let d = fed(&[(b"x", T0)]);
        let s = Session {
            state: SessionState::Exited,
            exit_code: None,
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(s), T0),
            SlotState::Lost { reclaimable: false }
        );
    }

    /// The predicate a grid and a shell both ask. They did not share one
    /// before, which is how a grid starts lying about what pressing it
    /// does.
    #[test]
    fn a_pad_is_free_when_nothing_live_holds_it() {
        assert!(SlotState::Empty.frees_the_pad());
        assert!(SlotState::Failed { code: 1 }.frees_the_pad());
        assert!(SlotState::Lost { reclaimable: true }.frees_the_pad());
        assert!(!SlotState::Lost { reclaimable: false }.frees_the_pad());
        assert!(!SlotState::Working.frees_the_pad());
        assert!(!SlotState::Quiet.frees_the_pad());
        assert!(!SlotState::NeedsAnswer {
            source: Source::Declared
        }
        .frees_the_pad());
    }

    /// `lost` usually means *anchor cannot account for this*, which
    /// includes "it may still be running". A daemon restart is the one
    /// reason that cannot mean that (#9).
    #[test]
    fn a_daemon_restart_is_the_one_lost_that_means_gone() {
        let d = Derived::default();
        let restarted = Session {
            state: SessionState::Lost,
            lost_to_restart: true,
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(restarted), T0),
            SlotState::Lost { reclaimable: true }
        );
        let unknown = Session {
            state: SessionState::Lost,
            lost_to_restart: false,
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(unknown), T0),
            SlotState::Lost { reclaimable: false }
        );
    }

    #[test]
    fn lost_is_not_a_clean_exit() {
        let d = fed(&[(b"x", T0)]);
        let s = Session {
            state: SessionState::Lost,
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(s), T0),
            SlotState::Lost { reclaimable: false }
        );
    }

    /// Closing is still alive and has no exit code; rendering it as Exited
    /// would mean inventing one.
    #[test]
    fn closing_still_renders_as_live() {
        let d = fed(&[(b"x", T0)]);
        let s = Session {
            state: SessionState::Closing,
            exit_code: None,
            ..live(&d)
        };
        assert_eq!(slot_state(Some(s), T0 + 1_000), SlotState::Working);
    }

    #[test]
    fn a_declared_signal_overrides_a_quiet_stream() {
        let d = fed(&[(b"busy", T0)]);
        let s = Session {
            declared_attention: true,
            ..live(&d)
        };
        // No bell anywhere in the stream, and it spoke a moment ago.
        assert!(!d.needs_answer(T0 + 1_000));
        assert_eq!(
            slot_state(Some(s), T0 + 1_000),
            SlotState::NeedsAnswer {
                source: Source::Declared
            }
        );
    }

    #[test]
    fn a_declared_signal_survives_its_source_going_quiet() {
        let d = fed(&[(b"busy", T0)]);
        let s = Session {
            declared_attention: true,
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(s), T0 + 600_000),
            SlotState::NeedsAnswer {
                source: Source::Declared
            }
        );
    }

    /// Mute falls through rather than short-circuiting: a muted agent is
    /// still an agent you can see is alive.
    #[test]
    fn mute_suppresses_attention_and_nothing_else() {
        let d = fed(&[(b"?\x07", T0)]);
        let derived_amber = Session {
            muted: true,
            ..live(&d)
        };
        assert_eq!(
            slot_state(Some(derived_amber), T0 + 3_000),
            SlotState::Working
        );

        let declared = Session {
            muted: true,
            declared_attention: true,
            ..live(&d)
        };
        assert_eq!(slot_state(Some(declared), T0 + 3_000), SlotState::Working);

        let long_quiet = Session {
            muted: true,
            ..live(&d)
        };
        assert_eq!(slot_state(Some(long_quiet), T0 + 60_000), SlotState::Quiet);
    }

    /// A freshly created session with no watcher yet is alive and idle --
    /// green and still. There is no separate "starting" state: off-to-green
    /// is already the feedback that the press landed.
    #[test]
    fn a_session_with_no_watcher_yet_is_alive_and_idle() {
        let empty = Derived {
            last_output_ms: T0,
            ..Derived::default()
        };
        let s = Session {
            derived: None,
            ..live(&empty)
        };
        assert_eq!(slot_state(Some(s), T0), SlotState::Quiet);
    }

    /// A session that rings before emitting anything else is still amber.
    #[test]
    fn a_bell_before_any_other_output_still_asks() {
        let d = fed(&[(b"\x07", T0)]);
        assert_eq!(
            slot_state(Some(live(&d)), T0 + 3_000),
            SlotState::NeedsAnswer {
                source: Source::Derived
            }
        );
    }

    #[test]
    fn working_becomes_quiet_at_the_idle_threshold() {
        let d = fed(&[(b"tokens", T0)]);
        let s = live(&d);
        assert_eq!(
            slot_state(Some(s), T0 + IDLE_THRESHOLD_MS - 1),
            SlotState::Working
        );
        assert_eq!(
            slot_state(Some(s), T0 + IDLE_THRESHOLD_MS),
            SlotState::Quiet
        );
    }
}
