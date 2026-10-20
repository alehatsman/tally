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

use std::path::Path;
use std::process::Command;

/// `cargo:` directives, and nothing else.
fn main() {
    // Emitting *any* `rerun-if-changed` replaces cargo's default scan of
    // the package, so this list is the whole of what re-runs the script.
    // Both halves of the answer need watching: the hash moves with git,
    // and the `+` moves with the tree -- a git-only list would let an
    // edited worktree keep a hash that says clean, which is the one
    // direction a build id must never be wrong in.
    for p in ["src", "Cargo.toml"] {
        println!("cargo:rerun-if-changed={p}");
    }
    // `--git-path`, not `.git/...`: in a linked worktree `.git` is a
    // file and those paths do not exist. A `rerun-if-changed` target
    // that is missing is not inert -- cargo calls the script dirty and
    // re-runs it on every build, forking two `git` processes each time.
    for f in ["HEAD", "index", "packed-refs"] {
        // `packed-refs` is absent in a repo that has never packed; a
        // path git names but that is not there yet is skipped for the
        // same reason.
        let Some(path) = git(&["rev-parse", "--git-path", f]) else {
            continue;
        };
        if Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
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
