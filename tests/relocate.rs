//! What tmux does, against real tmux.
//!
//! The unit tests in `src/jump.rs` pin what tally **decides**:
//! `relocation()` as a pure function of a listing, and the argument lists
//! it produces. Everything `docs/31-the-moved-view.md` argues from is a
//! fact about what tmux **does** — that `move-window -k` onto a corpse is
//! atomic, that moving a row's last window destroys the session, where a
//! placeholder lands, what a close does to the indexes beside it. A
//! fixture listing cannot be wrong about any of those, because a human
//! typed it.
//!
//! [#20](https://github.com/alehatsman/tally/issues/20) is the proof that
//! class of bug exists: every unit test passed with `renumber-windows on`
//! set on the live server, because no unit test has a server. The only
//! reason it was found is that somebody ran the gate by hand.
//!
//! Filed as [#21](https://github.com/alehatsman/tally/issues/21); the
//! scenarios are the eight `docs/31` names, plus the two that issue's
//! neighbours added.
//!
//! **This is the first thing in the repo that needs tmux installed.** It
//! skips loudly when it is absent rather than failing — see
//! `harness::skip` for why that is uncomfortable and said out loud.

#![allow(clippy::panic, reason = "a failed assertion is how a test reports")]

mod harness;

use harness::each_config;
use tally::jump::{Jump, Place};

/// `Place` for a row and column, which is all these tests ever build.
const fn at(row: u8, col: u8) -> Place {
    Place { row, col }
}

/// The address string a pad carries for row/col on page 1.
fn addr(row: u8, col: u8) -> String {
    format!("1:{row}:{col}")
}

/// Press a pad: make the view if it is not there, move it if it is out of
/// place, and label it. The whole of `view_pane`, through the one public
/// door it has.
fn press(server: &harness::Server, session: &str, row: u8, col: u8) {
    server
        .tmux()
        .jump(session, None, &addr(row, col), at(row, col))
        .unwrap_or_else(|e| {
            panic!(
                "press {session}@{row}:{col} failed: {e} — {}",
                server.dump()
            )
        });
}

/// Move a pad without looking at it: what `tallyd` runs after a Move.
fn moved(server: &harness::Server, session: &str, row: u8, col: u8) -> Result<(), String> {
    server
        .tmux()
        .relocate(session, &addr(row, col), at(row, col))
        .map_err(|e| e.to_string())
}

// ── the moves ──────────────────────────────────────────────────────────────

/// The commonest Move there is, and the one the live gate ran first.
#[test]
fn a_move_within_a_row_leaves_the_view_at_the_window_its_label_names() {
    each_config("move within a row", |s| {
        press(s, "S1", 1, 1);
        press(s, "S2", 1, 3);

        moved(s, "S1", 1, 2).expect("relocate");

        let v = s.view("S1");
        assert!(v.at("r01", 2), "S1 should be at r01:2 — {}", s.dump());
        assert_eq!(v.slot, "1:1:2", "the label follows the pane — {}", s.dump());
        assert!(
            s.view("S2").at("r01", 3),
            "S2 must not have been disturbed — {}",
            s.dump()
        );
    });
}

/// A move to a row tmux already holds: `move-window` and nothing else.
#[test]
fn a_move_to_a_row_that_exists_lands_in_it() {
    each_config("move to an existing row", |s| {
        press(s, "S1", 1, 1);
        press(s, "S2", 2, 1);

        moved(s, "S1", 2, 2).expect("relocate");

        assert!(
            s.view("S1").at("r02", 2),
            "S1 should be at r02:2 — {}",
            s.dump()
        );
        assert!(s.view("S2").at("r02", 1), "S2 stays — {}", s.dump());
    });
}

/// **The branch `docs/31` called the riskiest thing in the change**, and
/// the one M18's live gate never closed: tmux cannot adopt a pane into a
/// session it is creating, so a placeholder is made, moved onto, and
/// killed.
///
/// It also pins the second half of that sequence: moving a row's **last**
/// window out destroys the source session, and nothing should be left
/// behind when it does.
#[test]
fn a_move_to_a_row_that_does_not_exist_creates_it_and_leaves_no_placeholder() {
    each_config("move to a new row", |s| {
        press(s, "S1", 1, 1);
        assert!(s.session_exists("r01"), "r01 should exist — {}", s.dump());

        moved(s, "S1", 5, 3).expect("relocate");

        let v = s.view("S1");
        assert!(v.at("r05", 3), "S1 should be at r05:3 — {}", s.dump());
        assert!(
            !s.session_exists("r01"),
            "moving a row's last window out destroys it — {}",
            s.dump()
        );
        let strays = s
            .panes()
            .into_iter()
            .filter(|p| p.tmux_session == "r05" && p.session.is_empty())
            .count();
        assert_eq!(strays, 0, "the placeholder must be gone — {}", s.dump());
    });
}

/// A corpse holds the destination. `-k` replaces it in one step, which is
/// the thing `kill-window` then `move-window` is not.
#[test]
fn a_move_onto_a_corpse_replaces_it() {
    each_config("move onto a corpse", |s| {
        press(s, "S1", 1, 1);
        press(s, "S2", 1, 2);

        // Kill the command, not the pane: `remain-on-exit` is already set
        // on it by `tag`, so what is left at r01:2 is a corpse.
        let corpse = s.view("S2").id;
        let _respawned = s.sh(&["respawn-pane", "-k", "-t", &corpse, "true"]);
        s.wait_until_dead(&corpse);

        moved(s, "S1", 1, 2).expect("relocate");

        assert!(
            s.view("S1").at("r01", 2),
            "S1 should have taken r01:2 — {}",
            s.dump()
        );
        assert!(
            !s.panes().iter().any(|p| p.id == corpse),
            "the corpse should be gone — {}",
            s.dump()
        );
    });
}

/// Two live views trade places (`docs/34`,
/// [#19](https://github.com/alehatsman/tally/issues/19)).
///
/// ⚠️ **This test used to assert the opposite.** It pinned the hole
/// `docs/31` shipped knowingly — the view left where it stood and
/// labelled where it was not — and was written to fail the day the hole
/// was closed. This is that day; it is rewritten rather than deleted,
/// because the old assertion is the thing worth being able to see in the
/// history.
///
/// The arrangement is the one that actually happens. Move's legal target
/// is a pad with **no live session** (`docs/22`), so a live occupant is
/// by construction a view that is itself out of place — which is what
/// [#20](https://github.com/alehatsman/tally/issues/20) produced in bulk
/// before `hold_indexes` pinned it.
#[test]
fn two_live_views_trade_places() {
    each_config("swap with a live occupant", |s| {
        // S2 belongs at 1:1:5 and is drifted onto 1:1:2's window, the way
        // a close with `renumber-windows on` used to do it.
        press(s, "S2", 1, 5);
        let squatter = s.view("S2").id;
        let _drifted = s.sh(&["move-window", "-s", &squatter, "-t", "r01:2"]);
        assert_eq!(
            s.view("S2").slot,
            "1:1:5",
            "still claims 1:1:5 — {}",
            s.dump()
        );

        // S1 is moved onto the address S2 is squatting on.
        press(s, "S1", 1, 1);
        moved(s, "S1", 1, 2).expect("relocate");

        let p = s.view("S1");
        assert!(p.at("r01", 2), "S1 lands at its address — {}", s.dump());
        assert_eq!(p.slot, "1:1:2", "and wears it — {}", s.dump());

        let o = s.view("S2");
        assert!(o.at("r01", 1), "S2 took S1's old window — {}", s.dump());
        assert_eq!(
            o.id,
            squatter,
            "the same pane, not a new one — {}",
            s.dump()
        );
        assert!(!o.dead, "nothing was closed to make room — {}", s.dump());
        assert_eq!(o.slot, "1:1:5", "its own claim is untouched — {}", s.dump());
    });
}

/// The displaced view is not thereby correct, and the swap does not
/// pretend it is — but it is no worse off than any other drift, and the
/// lazy path still brings it home.
///
/// This is the honest limit of `docs/34`: a swap chooses *which* view is
/// out of place, it does not reduce how many are. Draining the last one
/// without a press is [#22](https://github.com/alehatsman/tally/issues/22).
#[test]
fn the_view_a_swap_displaced_is_repaired_by_its_own_press() {
    each_config("swap then repair the displaced", |s| {
        press(s, "S2", 1, 5);
        let squatter = s.view("S2").id;
        let _drifted = s.sh(&["move-window", "-s", &squatter, "-t", "r01:2"]);
        press(s, "S1", 1, 1);
        moved(s, "S1", 1, 2).expect("relocate");

        press(s, "S2", 1, 5);

        assert!(
            s.view("S2").at("r01", 5),
            "pressing S2 brings it home — {}",
            s.dump()
        );
        assert!(s.view("S1").at("r01", 2), "S1 stays put — {}", s.dump());
    });
}

// ── the no-ops ─────────────────────────────────────────────────────────────

/// A session nobody has pressed has no pane to move, and that is not an
/// error (`docs/31`).
#[test]
fn relocating_a_session_with_no_view_is_not_an_error() {
    each_config("relocate with no view", |s| {
        moved(s, "never-pressed", 3, 4).expect("a missing view is not a failure");
        assert!(!s.has_view("never-pressed"), "nothing made — {}", s.dump());
    });
}

/// Already there: `move-window` onto its own index is refused by tmux, so
/// the plan has to be `None` rather than an attempt that fails quietly.
#[test]
fn relocating_a_view_that_is_already_in_place_moves_nothing() {
    each_config("relocate in place", |s| {
        press(s, "S1", 1, 1);
        let before = s.view("S1");

        moved(s, "S1", 1, 1).expect("relocate");

        assert_eq!(before, s.view("S1"), "nothing should change — {}", s.dump());
    });
}

// ── the pad it left, and the drift nothing looks for ───────────────────────

/// The symptom half of [#15](https://github.com/alehatsman/tally/issues/15):
/// a Move used to leave the pane behind, so the pad it vacated failed with
/// `create window failed: index 1 in use` for as long as the row lived.
#[test]
fn a_pad_a_move_vacated_can_be_spawned_onto() {
    each_config("spawn onto a vacated pad", |s| {
        press(s, "S1", 1, 1);
        moved(s, "S1", 1, 4).expect("relocate");

        press(s, "S3", 1, 1);

        assert!(s.view("S3").at("r01", 1), "S3 takes r01:1 — {}", s.dump());
        assert!(s.view("S1").at("r01", 4), "S1 stayed put — {}", s.dump());
    });
}

/// The lazy path: a view that drifted while `tallyd` was down heals on the
/// next press of **its own** pad.
///
/// It heals on no other, which is
/// [#22](https://github.com/alehatsman/tally/issues/22) and is why this
/// test drifts the pane by hand — a Move is not the only way to get here.
#[test]
fn pressing_a_drifted_view_relocates_it_by_id() {
    each_config("lazy relocation by id", |s| {
        press(s, "S1", 1, 1);
        let pane = s.view("S1").id;

        // Drift it behind tally's back, the way tmux itself can.
        let _moved = s.sh(&["move-window", "-s", &pane, "-t", "r01:6"]);
        assert!(s.view("S1").at("r01", 6), "arranged — {}", s.dump());

        press(s, "S1", 1, 1);

        let v = s.view("S1");
        assert!(v.at("r01", 1), "pressing it brings it home — {}", s.dump());
        assert_eq!(v.id, pane, "and it is the same pane — {}", s.dump());
    });
}

// ── #20, which is the reason the matrix exists ─────────────────────────────

/// The generator, at the tmux level, with tally kept out of it.
///
/// **Without this the regression guard below could pass for the wrong
/// reason** — a config file that was never read would look exactly like a
/// `hold_indexes` that works. This asserts the `-f` actually bit.
#[test]
fn renumber_windows_shifts_siblings_when_nothing_has_pinned_it() {
    each_config("renumber, unpinned", |s| {
        if !s.config.renumber {
            return;
        }
        // A session of its own, so nothing tally does touches it. No
        // `Tmux` is built in this test, so `hold_indexes` never runs.
        let _made = s.sh(&["new-session", "-d", "-s", "raw", "sleep 100000"]);
        for i in [2, 3, 4] {
            let _w = s.sh(&[
                "new-window",
                "-d",
                "-t",
                &format!("raw:{i}"),
                "sleep 100000",
            ]);
        }
        let _killed = s.sh(&["kill-window", "-t", "raw:2"]);

        let indexes: Vec<String> = s
            .panes()
            .into_iter()
            .filter(|p| p.tmux_session == "raw")
            .map(|p| p.index)
            .collect();
        assert!(
            !indexes.contains(&"4".to_owned()),
            "with renumber on, closing raw:2 should pull 4 down — got {indexes:?} under {}",
            s.config.name
        );
    });
}

/// [#20](https://github.com/alehatsman/tally/issues/20), as a guard.
///
/// The owner's `~/.tmux.conf` sets `renumber-windows on`; a named socket
/// is a separate *server* and not a separate *config*, so tally's server
/// inherited it and closing a tab shifted every view to its right down
/// one — a drifted view with no Move involved, which is the state
/// [#19](https://github.com/alehatsman/tally/issues/19)'s branch then
/// cannot repair. `Tmux::hold_indexes` pins the option off; this is the
/// only thing in the repo that can tell whether it does.
#[test]
fn tally_holds_the_indexes_still_even_when_the_config_renumbers() {
    each_config("renumber, pinned by tally", |s| {
        press(s, "S1", 1, 1);
        press(s, "S2", 1, 2);
        press(s, "S3", 1, 3);

        // The press that comes after a close is what re-pins it, but the
        // close is what renumbers — so the option has to be off *before*
        // this, which is what `hold_indexes` being called on every press
        // buys.
        let gone = s.view("S2").id;
        let window = s.sh(&["display-message", "-p", "-t", &gone, "#{window_id}"]);
        let _killed = s.sh(&["kill-window", "-t", &window]);

        assert!(
            s.view("S3").at("r01", 3),
            "S3 must still be at the column its address names — {}",
            s.dump()
        );
        assert!(s.view("S1").at("r01", 1), "and S1 at 1 — {}", s.dump());
    });
}

/// The whole of #20's damage, end to end: a close renumbers, the view to
/// its right is now at an index no address claims, and the next press of
/// the **vacated** pad meets `index N in use`.
///
/// With `hold_indexes` doing its job there is no drift to meet, so the
/// press simply works. That is the assertion.
#[test]
fn a_press_after_a_close_does_not_meet_index_in_use() {
    each_config("press after a close", |s| {
        press(s, "S1", 1, 1);
        press(s, "S2", 1, 2);
        press(s, "S3", 1, 3);

        let gone = s.view("S2").id;
        let window = s.sh(&["display-message", "-p", "-t", &gone, "#{window_id}"]);
        let _killed = s.sh(&["kill-window", "-t", &window]);

        // The pad the close vacated, pressed with a new session on it.
        press(s, "S4", 1, 2);

        assert!(s.view("S4").at("r01", 2), "S4 takes r01:2 — {}", s.dump());
        assert!(s.view("S3").at("r01", 3), "S3 unmoved — {}", s.dump());
    });
}
