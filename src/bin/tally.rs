//! `tally` — the CLI half of spawning.
//!
//! `slots.toml` answers "what does this pad run"; this answers "claim a pad
//! for what I am doing *here*". The directory is the whole reason it
//! exists: it is the one fact a config file cannot know in advance and a
//! shell always knows (`docs/11-cli.md`).
//!
//! It talks to **anchord**, never to tallyd. The grid is downstream of
//! anchor's state, so a label written here shows up on the device within
//! one reconcile and nothing needs a socket
//! (`docs/08-lifecycle.md#one-process-no-socket`).

use std::collections::BTreeMap;
use std::process::ExitCode;

use tally::anchor::{Client, Spawn};
use tally::attention::Ancestry;
use tally::cli::{self, Claim, Request, USAGE};
use tally::config::Config;
use tally::model;
use tally::signal::SlotState;
use tally::slot::Pages;

/// Bad typing. Conventional, and distinguishable by a script from a spawn
/// that was asked for correctly and failed anyway.
const USAGE_ERROR: u8 = 2;

fn main() -> ExitCode {
    let config = match Config::load(&Config::default_path()) {
        Ok(config) => config,
        // tallyd carries on with defaults here, because a grid that stops
        // lighting up over a typo is worse than one with unnamed pages.
        // A spawn is different: the page names decide where the session
        // lands, and guessing is how you lose one.
        Err(e) => {
            eprintln!("tally: {}: {e}", Config::default_path().display());
            return ExitCode::FAILURE;
        }
    };
    let pages = config.pages();

    match cli::parse(std::env::args().skip(1), pages) {
        Ok(Request::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Request::Spawn(claim)) => spawn(&claim, pages),
        Ok(Request::Attention(on)) => attention(on),
        Err(e) => {
            eprint!("{USAGE}");
            eprintln!("\ntally: {e}");
            ExitCode::from(USAGE_ERROR)
        }
    }
}

/// Set or remove `tl.attention` on whichever session we are inside.
///
/// **Not being under anchor is success**, and that is the whole shape of
/// this function. It is called from a Claude Code hook, which fires on
/// every turn of every agent on this machine; all but a handful are not on
/// the grid. An error there would put a line in the user's own agent, every
/// turn, for a surface that is not even watching it.
///
/// Nothing is ever written to stdout. stdout belongs to the agent's
/// transcript, and this is not something the agent said.
fn attention(on: bool) -> ExitCode {
    let client = Client::new(Client::default_path());
    let sessions = match client.list() {
        Ok(sessions) => sessions,
        // A real failure, and the one case that says anything. With anchord
        // down we cannot tell "not under anchor" from "could not look", so
        // we report rather than guess — see `docs/11-cli.md` for why the
        // hook line ends in `|| true` regardless.
        Err(e) => {
            eprintln!("tally: {e}");
            return ExitCode::FAILURE;
        }
    };

    let ancestry = Ancestry::read();
    let me = std::process::id().cast_signed();
    let Some(session) = tally::attention::owning_session(&sessions, me, &ancestry) else {
        return ExitCode::SUCCESS;
    };

    let mut set = BTreeMap::new();
    let remove: &[&str] = if on {
        let _on = set.insert(tally::anchor::LABEL_ATTENTION.to_owned(), "1".to_owned());
        &[]
    } else {
        &[tally::anchor::LABEL_ATTENTION]
    };
    match client.label(&session.id, &set, remove) {
        Ok(()) => ExitCode::SUCCESS,
        // The session ended between the list and the write. Nothing to
        // label, nothing wrong: a pad for a session that is gone is a pad
        // that renders its exit.
        Err(tally::anchor::Error::Refused { ref code, .. }) if code == "not_found" => {
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("tally: {e}");
            ExitCode::FAILURE
        }
    }
}

fn spawn(claim: &Claim, pages: &Pages) -> ExitCode {
    let Some(address) = claim.at.label(pages) else {
        eprintln!("tally: that address has no page name");
        return ExitCode::FAILURE;
    };
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("tally: cannot read the current directory: {e}");
            return ExitCode::FAILURE;
        }
    };

    let client = Client::new(Client::default_path());
    let sessions = match client.list() {
        Ok(sessions) => sessions,
        Err(e) => {
            eprintln!("tally: {e}");
            return ExitCode::FAILURE;
        }
    };

    // The same predicate the grid draws with, run without watchers: a pad
    // showing something refuses, a dark one spawns. If these two ever
    // disagree, the grid is lying about what pressing it does.
    let drawn = model::resolve(&sessions, &BTreeMap::new(), pages, tally::now_ms());
    if let Some(held) = drawn.get(&claim.at) {
        eprintln!(
            "tally: {address} is held by {} ({})",
            held.session,
            describe(held.state)
        );
        return ExitCode::FAILURE;
    }

    let mut labels = BTreeMap::new();
    let _slot = labels.insert(tally::anchor::LABEL_SLOT.to_owned(), address.clone());
    // No `tl.spec`: a spec label is a name looked up in `slots.toml`, and
    // one that nothing declares would be a label that lies. The cost is a
    // dead respawn chord on this pad (`docs/11-cli.md`).
    match client.create(&Spawn {
        command: &claim.command,
        args: &claim.args,
        cwd: Some(&cwd),
        env: &BTreeMap::new(),
        login_shell: true,
        labels: &labels,
    }) {
        Ok(id) => {
            println!("spawned {id} on {address}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("tally: {e}");
            ExitCode::FAILURE
        }
    }
}

/// What to call a state in a sentence. The pad says it in colour; this
/// has to say it in words, and "Failed { code: 1 }" is not words.
fn describe(state: SlotState) -> String {
    match state {
        SlotState::Empty => "nothing".to_owned(),
        SlotState::Working => "running".to_owned(),
        SlotState::Quiet => "running, idle".to_owned(),
        SlotState::NeedsAnswer { .. } => "waiting for an answer".to_owned(),
        SlotState::Failed { code } => format!("failed, exit {code}"),
        SlotState::Lost => "lost".to_owned(),
    }
}
