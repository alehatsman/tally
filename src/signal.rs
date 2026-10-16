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

const BEL: u8 = 0x07;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    Unbound,
    Bound,
    Starting,
    Working,
    Quiet,
    NeedsAnswer { source: Source },
    Exited { code: i32 },
    /// anchord restarted. The process may well still be running, orphaned —
    /// rendering that as a clean exit lies at exactly the wrong moment.
    Lost,
}

/// anchor's view of a session's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Running,
    /// Termination requested; the child has not been reaped yet. Still
    /// **alive**, and it has no exit code to render.
    Closing,
    Exited,
    Lost,
}

/// Signals scraped from one session's output stream.
#[derive(Debug, Clone, Default)]
pub struct Derived {
    /// When a BEL was last seen. `None` if never.
    pub last_bell_ms: Option<i64>,
    /// When output last arrived.
    pub last_output_ms: i64,
    /// One past the last byte scanned, so a watcher restarted after being
    /// dropped for backpressure does not re-read the whole log.
    pub scanned_to: u64,
    /// A bell is outstanding: it rang and has not spoken since.
    bell_pending: bool,
}

impl Derived {
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
        let tail = match bytes.iter().rposition(|&b| b == BEL) {
            Some(i) => {
                self.last_bell_ms = Some(now);
                self.bell_pending = true;
                &bytes[i + 1..]
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

    /// Nothing for [`IDLE_THRESHOLD_MS`].
    pub fn idle(&self, now: i64) -> bool {
        now - self.last_output_ms >= IDLE_THRESHOLD_MS
    }
}

/// Everything about a session that the state decision needs.
#[derive(Debug, Clone, Copy)]
pub struct Session<'a> {
    pub state: SessionState,
    pub exit_code: Option<i32>,
    /// From `list`; a session with no bytes yet needs no watcher to be
    /// known as `Starting`.
    pub output_bytes: u64,
    /// `tl.mute` — suppresses attention, nothing else.
    pub muted: bool,
    /// `tl.attention` — a hook declared this session is waiting.
    pub declared_attention: bool,
    /// `None` when no watcher has produced anything yet.
    pub derived: Option<&'a Derived>,
}

/// The ordered match that is the whole opinion (`docs/04-signals.md`).
///
/// `session` is `None` when no session holds the slot; `has_spec` says
/// whether `slots.toml` could put one there.
pub fn slot_state(session: Option<Session<'_>>, has_spec: bool, now: i64) -> SlotState {
    let Some(s) = session else {
        return if has_spec { SlotState::Bound } else { SlotState::Unbound };
    };

    match s.state {
        // Terminal states outrank attention: an agent that rang and then
        // died is Exited, not amber. There is nothing left to answer.
        SessionState::Lost => return SlotState::Lost,
        SessionState::Exited => return SlotState::Exited { code: s.exit_code.unwrap_or(-1) },
        // Closing is still alive and has no exit code yet, so it falls
        // through to the live states below.
        SessionState::Running | SessionState::Closing => {}
    }

    if !s.muted {
        if s.declared_attention {
            return SlotState::NeedsAnswer { source: Source::Declared };
        }
        if s.derived.is_some_and(|d| d.needs_answer(now)) {
            return SlotState::NeedsAnswer { source: Source::Derived };
        }
    }

    // Below attention, so a session that rings before emitting anything
    // else is still amber.
    if s.output_bytes == 0 {
        return SlotState::Starting;
    }
    match s.derived {
        Some(d) if d.idle(now) => SlotState::Quiet,
        Some(_) => SlotState::Working,
        None => SlotState::Quiet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_760_000_000_000;

    fn fed(chunks: &[(&[u8], i64)]) -> Derived {
        let mut d = Derived { last_output_ms: T0, ..Derived::default() };
        for (bytes, at) in chunks {
            d.feed(bytes, *at);
        }
        d
    }

    fn live(derived: &Derived) -> Session<'_> {
        Session {
            state: SessionState::Running,
            exit_code: None,
            output_bytes: 42,
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
        assert!(same_chunk.needs_answer(T0 + 3_000), "trailing newline cancelled it");

        let next_chunk = fed(&[(b"ready?\x07", T0), (b"\r\n", T0 + 50)]);
        assert!(next_chunk.needs_answer(T0 + 3_000), "newline in the next chunk cancelled it");

        let spaces = fed(&[(b"\x07", T0), (b"   \t\n", T0 + 10)]);
        assert!(spaces.needs_answer(T0 + 3_000));
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
        assert_eq!(std::mem::size_of_val(&d), std::mem::size_of::<Derived>());
    }

    #[test]
    fn scanned_to_advances_so_a_restarted_watcher_resumes() {
        let d = fed(&[(b"abc", T0), (b"defg", T0 + 1)]);
        assert_eq!(d.scanned_to, 7);
    }

    // --- precedence ---------------------------------------------------

    #[test]
    fn an_empty_slot_is_unbound_or_bound() {
        assert_eq!(slot_state(None, false, T0), SlotState::Unbound);
        assert_eq!(slot_state(None, true, T0), SlotState::Bound);
    }

    #[test]
    fn a_bell_then_exit_is_exited_not_amber() {
        let d = fed(&[(b"?\x07", T0)]);
        let s = Session { state: SessionState::Exited, exit_code: Some(0), ..live(&d) };
        assert_eq!(slot_state(Some(s), true, T0 + 5_000), SlotState::Exited { code: 0 });
    }

    #[test]
    fn lost_is_not_a_clean_exit() {
        let d = fed(&[(b"x", T0)]);
        let s = Session { state: SessionState::Lost, ..live(&d) };
        assert_eq!(slot_state(Some(s), true, T0), SlotState::Lost);
    }

    /// Closing is still alive and has no exit code; rendering it as Exited
    /// would mean inventing one.
    #[test]
    fn closing_still_renders_as_live() {
        let d = fed(&[(b"x", T0)]);
        let s = Session { state: SessionState::Closing, exit_code: None, ..live(&d) };
        assert_eq!(slot_state(Some(s), true, T0 + 1_000), SlotState::Working);
    }

    #[test]
    fn a_declared_signal_overrides_a_quiet_stream() {
        let d = fed(&[(b"busy", T0)]);
        let s = Session { declared_attention: true, ..live(&d) };
        // No bell anywhere in the stream, and it spoke a moment ago.
        assert!(!d.needs_answer(T0 + 1_000));
        assert_eq!(
            slot_state(Some(s), true, T0 + 1_000),
            SlotState::NeedsAnswer { source: Source::Declared }
        );
    }

    #[test]
    fn a_declared_signal_survives_its_source_going_quiet() {
        let d = fed(&[(b"busy", T0)]);
        let s = Session { declared_attention: true, ..live(&d) };
        assert_eq!(
            slot_state(Some(s), true, T0 + 600_000),
            SlotState::NeedsAnswer { source: Source::Declared }
        );
    }

    /// Mute falls through rather than short-circuiting: a muted agent is
    /// still an agent you can see is alive.
    #[test]
    fn mute_suppresses_attention_and_nothing_else() {
        let d = fed(&[(b"?\x07", T0)]);
        let derived_amber = Session { muted: true, ..live(&d) };
        assert_eq!(slot_state(Some(derived_amber), true, T0 + 3_000), SlotState::Working);

        let declared = Session { muted: true, declared_attention: true, ..live(&d) };
        assert_eq!(slot_state(Some(declared), true, T0 + 3_000), SlotState::Working);

        let long_quiet = Session { muted: true, ..live(&d) };
        assert_eq!(slot_state(Some(long_quiet), true, T0 + 60_000), SlotState::Quiet);
    }

    /// Below attention, so a session that rings before saying anything else
    /// is still amber.
    #[test]
    fn starting_needs_no_watcher_but_yields_to_attention() {
        let empty = Derived { last_output_ms: T0, ..Derived::default() };
        let s = Session { output_bytes: 0, derived: None, ..live(&empty) };
        assert_eq!(slot_state(Some(s), true, T0), SlotState::Starting);

        let d = fed(&[(b"\x07", T0)]);
        let rang = Session { output_bytes: 0, ..live(&d) };
        assert_eq!(
            slot_state(Some(rang), true, T0 + 3_000),
            SlotState::NeedsAnswer { source: Source::Derived }
        );
    }

    #[test]
    fn working_becomes_quiet_at_the_idle_threshold() {
        let d = fed(&[(b"tokens", T0)]);
        let s = live(&d);
        assert_eq!(slot_state(Some(s), true, T0 + IDLE_THRESHOLD_MS - 1), SlotState::Working);
        assert_eq!(slot_state(Some(s), true, T0 + IDLE_THRESHOLD_MS), SlotState::Quiet);
    }
}
