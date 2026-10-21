//! The window a jump opens when nobody is looking, against real tmux.
//!
//! The unit tests in `src/jump.rs` pin what `terminal_command` builds from
//! a pane id it is handed. Nothing there can see which pane id `jump`
//! hands it, and that is the half that regressed: a window attached to the
//! home session sat on a shell until a second press (`docs/06-views.md`).
//!
//! `TALLY_TERMINAL_CMD` is pointed at a script that writes down its argv,
//! so the window this opens is a line in a file and not one on your screen.

#![allow(clippy::panic, reason = "a failed assertion is how a test reports")]

mod harness;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use harness::each_config;
use tally::jump::{Jump, Place};

/// A launcher that opens nothing and records what it was asked to run,
/// one argument per line.
fn recorder(dir: &std::path::Path) -> (PathBuf, PathBuf) {
    let script = dir.join("open-terminal");
    let argv = dir.join("argv");
    std::fs::write(
        &script,
        format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\n", argv.display()),
    )
    .expect("write recorder");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("chmod recorder");
    (script, argv)
}

/// Zero clients: the window is opened, on this server, attached to the
/// view the press made — not to the home session.
#[test]
fn a_cold_jump_opens_the_window_on_the_destination_pane() {
    each_config(
        "a_cold_jump_opens_the_window_on_the_destination_pane",
        |server| {
            let dir = std::env::temp_dir().join(format!("tally-cold-open-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("temp dir");
            let (script, argv) = recorder(&dir);
            // `harness::Server` holds the gate, so nothing else is reading this
            // while it is set, and the next server sets it back to `true`.
            std::env::set_var("TALLY_TERMINAL_CMD", &script);

            server
                .tmux()
                .jump("s1", None, "work:1:1", Place { row: 1, col: 1 }, None)
                .unwrap_or_else(|e| panic!("press failed: {e} — {}", server.dump()));
            std::env::set_var("TALLY_TERMINAL_CMD", "true");

            let recorded = std::fs::read_to_string(&argv).unwrap_or_else(|e| {
                panic!("no window was opened ({e}) under {}", server.config.name)
            });
            let args: Vec<&str> = recorded.lines().collect();
            let view = server.view("s1");
            assert_eq!(
                args.last().copied(),
                Some(view.id.as_str()),
                "the window attaches to the view, not the home shell; argv {args:?}"
            );
            assert_eq!(args.first().copied(), Some("tmux"), "argv {args:?}");
            let _cleaned = std::fs::remove_dir_all(&dir);
        },
    );
}
