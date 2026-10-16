//! How fast the surface breathes.
//!
//! The grid says *something wants you*. It has never said *and it has been
//! waiting eighteen minutes*, which is the thing that decides whether you
//! get up — and in real use the waits span two orders of magnitude (p50
//! **187 s**, p90 **1,076 s**, max **2,403 s**) while the pad looks
//! identical at both ends (`docs/17-the-tempo.md`).
//!
//! There is no second amber to spend on this. There is a clock.
//! `docs/02-surface.md` records that both animations run at 120 bpm *when
//! no MIDI clock is supplied*; supply one and the beat is ours while the
//! **ramp stays on the device**, so a pulse is still smooth under load and
//! still never stutters.
//!
//! ✅ **Confirmed on hardware** against a live grid: clock on
//! `LPMiniMK3 MIDI` — the port tally already opens, not the DAW port —
//! visibly slows and quickens the pulse. That was the one sentence the
//! whole milestone rested on and it was the manual's implication, not its
//! claim.
//!
//! This module is pure, like [`crate::queue`]: an age in, a tempo out.

use std::time::Duration;

/// Pulses per quarter note. The MIDI spec's number, not a choice.
pub const PPQN: u64 = 24;

/// The calm floor, in bpm.
///
/// **Slower than the device's own 120, and deliberately not slower still.**
/// A stopped clock is a thing you must be able to tell from a calm one:
/// if `tallyd` wedges, the device stops breathing mid-ramp, which is a
/// liveness signal the grid has never had ([`crate::tempo`] docs). At 60
/// bpm a pulse is a two-second ramp — unmistakably moving, unmistakably
/// unhurried.
pub const FLOOR_BPM: u64 = 60;

/// The fast end, in bpm.
///
/// Barely above the device's unsynced 120. The failure this project has
/// already paid for once is a surface that is correct and **exhausting**:
/// the hard amber flash read as urgent on hardware and as stressful after
/// a few hours (`docs/05-interaction.md`). A tempo that sprints is that
/// mistake with a different knob.
pub const CEILING_BPM: u64 = 140;

/// The age at which the tempo stops climbing, in milliseconds.
///
/// Fifteen minutes, from the observed distribution rather than a round
/// number: p75 is 335 s and p90 is 1,076 s, so the knee sits where real
/// waits stop being ordinary.
pub const KNEE_MS: i64 = 900_000;

/// The tempo for the oldest thing waiting.
///
/// `None` — nothing is asking — is the floor, not a special case: a calm
/// room and a room whose single request just arrived are equally calm, and
/// amber already says *look at me* without help from the beat.
#[must_use]
pub fn bpm(oldest_wait_ms: Option<i64>) -> u64 {
    let Some(age) = oldest_wait_ms else {
        return FLOOR_BPM;
    };
    let age = age.clamp(0, KNEE_MS);
    let span = CEILING_BPM - FLOOR_BPM;
    // i64 throughout: age and KNEE_MS are milliseconds and the product is
    // at most 900_000 * 80, nowhere near overflow.
    let climbed = (age * span.cast_signed()) / KNEE_MS;
    FLOOR_BPM + climbed.cast_unsigned()
}

/// How long between clock pulses at this tempo.
#[must_use]
pub fn gap(bpm: u64) -> Duration {
    Duration::from_nanos(60_000_000_000 / (bpm.max(1) * PPQN))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stopped clock has to be tellable from a calm one, and the floor
    /// is the whole defence.
    #[test]
    fn nothing_waiting_is_calm_and_not_stopped() {
        assert_eq!(bpm(None), FLOOR_BPM);
        const { assert!(FLOOR_BPM >= 60, "slower than this reads as broken") };
    }

    /// Something that just asked is not old. Amber says *look*; the beat
    /// says *how long*, and the answer is "no time at all".
    #[test]
    fn a_fresh_request_does_not_move_the_tempo() {
        assert_eq!(bpm(Some(0)), FLOOR_BPM);
        assert_eq!(bpm(Some(-5_000)), FLOOR_BPM, "a clock skew is not urgency");
    }

    #[test]
    fn the_tempo_climbs_with_age_and_then_stops() {
        let mut last = bpm(Some(0));
        for minutes in 1..=20 {
            let now = bpm(Some(minutes * 60_000));
            assert!(now >= last, "{minutes}m went backwards");
            last = now;
        }
        assert_eq!(bpm(Some(KNEE_MS)), CEILING_BPM);
        assert_eq!(bpm(Some(KNEE_MS * 10)), CEILING_BPM, "clamped, not faster");
    }

    /// The numbers in `docs/17` are a decision. Changing them should fail
    /// a test rather than drift.
    #[test]
    fn the_mapping_is_the_one_the_spec_states() {
        assert_eq!((FLOOR_BPM, CEILING_BPM, KNEE_MS), (60, 140, 900_000));
        // p50 of the observed waits, ~3 minutes.
        assert_eq!(bpm(Some(187_000)), 76);
        // p75, ~5.5 minutes.
        assert_eq!(bpm(Some(335_000)), 89);
        // p90, ~18 minutes -- past the knee.
        assert_eq!(bpm(Some(1_076_000)), CEILING_BPM);
    }

    /// 24 pulses per quarter note is the spec's number. At 60 bpm a
    /// quarter note is one second, so the gap is exactly 1/24 s.
    #[test]
    fn a_quarter_note_is_twenty_four_pulses() {
        assert_eq!(gap(60), Duration::from_nanos(1_000_000_000 / 24));
        let pulses_per_second = |b| 1.0 / gap(b).as_secs_f64();
        assert!((pulses_per_second(60) - 24.0).abs() < 0.001);
        assert!((pulses_per_second(140) - 56.0).abs() < 0.01);
    }

    /// The fast end must stay inside the loop's tick budget, or the beat
    /// is jittered by the thing sending it.
    #[test]
    fn the_ceiling_fits_in_a_sixteen_millisecond_tick() {
        assert!(
            gap(CEILING_BPM) > Duration::from_millis(16),
            "the main loop could not keep up with this"
        );
    }

    #[test]
    fn a_nonsense_tempo_does_not_divide_by_zero() {
        assert!(gap(0) > Duration::ZERO);
    }
}
