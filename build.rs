//! Stamps the git hash this binary was built from into `TALLY_BUILD`.
//!
//! **Why a build script for seven characters.** `tallyd` logs to stderr,
//! and the log is the only instrument this project has for the jump
//! subsystem. A measurement over a log that cannot say which binary wrote
//! it is not a measurement: 77 `index N in use` lines were refused as
//! evidence for exactly that reason (`ROADMAP.md`), and the sweep this
//! supports is meant to be read over a week that may span several builds
//! (`docs/06-views.md`).
//!
//! No dependency — `git` is already required to clone this, and when it
//! is not there to ask the answer is `unknown`, which is honest and not a
//! failure.

use std::process::Command;

/// `cargo:` directives, and nothing else.
fn main() {
    // A commit changes `HEAD` (or what it points at, via `packed-refs`);
    // staging changes `index`. A tree dirtied without staging is **not**
    // caught -- cargo has no way to watch the whole worktree, and the
    // `+` below can therefore be stale in one direction only: it can say
    // clean when the files have since been edited. Touch a source file
    // and the normal rebuild fixes it.
    for f in ["HEAD", "index", "packed-refs"] {
        println!("cargo:rerun-if-changed=.git/{f}");
    }
    println!("cargo:rustc-env=TALLY_BUILD={}", build_id());
}

/// `ab12cd3`, with `+` when the tree had uncommitted changes.
fn build_id() -> String {
    let Some(hash) = git(&["rev-parse", "--short=7", "HEAD"]) else {
        return "unknown".to_owned();
    };
    // Empty output means nothing to commit. A failed call means we do not
    // know, and claiming clean would be the wrong guess.
    let dirty = git(&["status", "--porcelain"]).is_none_or(|s| !s.is_empty());
    if dirty {
        format!("{hash}+")
    } else {
        hash
    }
}

/// `git` output with the trailing newline off, or `None` if it did not run.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8(out.stdout).ok()?.trim().to_owned())
}
