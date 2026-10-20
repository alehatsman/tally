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
    //
    // The tree half is the files that are linked into the binaries: this
    // crate, the lockfile (`cargo update` changes the binary and dirties
    // the tree without touching `src`), and `bridge/`, a path dependency
    // (`Cargo.toml`, `tallybridge`) whose source compiles into `tallyd`.
    // NOT watched, so a clean-looking id can sit on an edited tree there:
    // `tests/`, `docs/`, `examples/` and the rest of the tree. None of
    // them reach the binary, so the id names the right code; only the `+`
    // lags until the next rebuild for some other reason.
    for p in ["src", "Cargo.toml", "Cargo.lock", "bridge"] {
        println!("cargo:rerun-if-changed={p}");
    }
    // `--git-path`, not `.git/...`: in a linked worktree `.git` is a
    // file and those paths do not exist. A `rerun-if-changed` target
    // that is missing is not inert -- cargo calls the script dirty and
    // re-runs it on every build, forking two `git` processes each time.
    //
    // The loose ref `HEAD` points at is the fourth path: `git reset
    // --soft` and `git branch -f <current>` rewrite only
    // `refs/heads/<branch>`, leaving `HEAD`, the index and `packed-refs`
    // as they were. Detached HEAD has no ref (`HEAD` itself comes back),
    // and `HEAD` is already watched. A branch that exists only in
    // `packed-refs` has no loose file to name; the first `reset` on it
    // creates one, and that one move goes unseen -- the one gap left.
    let mut watched = vec![
        "HEAD".to_owned(),
        "index".to_owned(),
        "packed-refs".to_owned(),
    ];
    if let Some(r) = git(&["rev-parse", "--symbolic-full-name", "HEAD"]) {
        if r != "HEAD" {
            watched.push(r);
        }
    }
    for f in &watched {
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
