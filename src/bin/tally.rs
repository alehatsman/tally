//! `tally` — the CLI half of spawning.
//!
//! `slots.toml` answers "what does this pad run"; this answers "claim a pad
//! for what I am doing *here*". The directory is the whole reason it
//! exists: it is the one fact a config file cannot know in advance and a
//! shell always knows (`docs/07-cli.md`).
//!
//! It talks to **anchord**, never to tallyd. The grid is downstream of
//! anchor's state, so a label written here shows up on the device within
//! one reconcile and nothing needs a socket
//! (`docs/08-daemon.md#one-process-no-socket`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tally::anchor::{Client, Spawn};
use tally::attention::Ancestry;
use tally::cli::{self, Claim, Fork, Request, USAGE};
use tally::config;
use tally::jump::{Jump, Tmux};
use tally::listing::{self, Caller};
use tally::model;
use tally::slot::{self, Pages, Slot};

/// Bad typing. Conventional, and distinguishable by a script from a spawn
/// that was asked for correctly and failed anyway.
const USAGE_ERROR: u8 = 2;

/// anchord, and everything it is holding.
///
/// The prologue of every subcommand but `help`, and the failure is the
/// same in all of them: with anchord down there is no grid, and an empty
/// list would read as "nothing is running" when what we have is "I cannot
/// tell" ([`tally::anchor::Error::Unreachable`]).
fn connected() -> Result<(Client, Vec<tally::anchor::Session>), ExitCode> {
    let client = Client::new(Client::default_path());
    match client.list() {
        Ok(sessions) => Ok((client, sessions)),
        Err(e) => {
            eprintln!("tally: {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

/// How an address is spelled, or the refusal for a page nothing names.
///
/// An undeclared page has no spelling, so there is no `tl.slot` to write
/// and nothing to print: the address the user typed cannot be said back
/// to them.
fn address_of(at: Slot, pages: &Pages) -> Result<String, ExitCode> {
    at.label(pages).ok_or_else(|| {
        eprintln!("tally: that address has no page name");
        ExitCode::FAILURE
    })
}

/// Run a subcommand, turning its early exits back into an [`ExitCode`].
///
/// The `?` in the helpers above needs a `Result` to return into; `main`
/// needs an `ExitCode`. This is the one line that joins them.
fn report(outcome: Result<ExitCode, ExitCode>) -> ExitCode {
    match outcome {
        Ok(code) | Err(code) => code,
    }
}

fn main() -> ExitCode {
    let pages = match config::load(&config::default_path()) {
        Ok(pages) => pages,
        // A spawn stops where tallyd carries on: the page names decide
        // where the session lands. Both halves of that are argued in
        // [`tally::config`].
        Err(e) => {
            eprintln!("tally: {}: {e}", config::default_path().display());
            return ExitCode::FAILURE;
        }
    };
    let pages = &pages;

    match cli::parse(std::env::args().skip(1), pages) {
        Ok(Request::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Request::Ls(json)) => report(ls(json, pages)),
        Ok(Request::Spawn(claim)) => report(spawn(&claim, pages)),
        Ok(Request::Fork(f)) => report(fork(&f, pages)),
        Ok(Request::Attention(wants)) => report(attention(wants)),
        Ok(Request::Purge(at)) => report(purge(at, pages)),
        Ok(Request::Mute(at, on)) => report(mute(at, on, pages)),
        Err(e) => {
            eprint!("{USAGE}");
            eprintln!("\ntally: {e}");
            ExitCode::from(USAGE_ERROR)
        }
    }
}

/// Print the grid, marking the pad this process is standing on.
///
/// **The one subcommand that writes nothing.** It is here because the
/// device has rendered sixty-four pads since M0 and never rendered one as
/// a character, and an agent on a pad cannot see colour: locating itself
/// by hand cost six tool calls before this existed
/// (`docs/07-cli.md`).
///
/// Unlike every other subcommand, **not being under anchor is not a
/// failure**. The grid is a thing you can look at from outside, and most
/// shells on this machine are outside; what is missing is the `@`, not
/// the answer.
fn ls(json: bool, pages: &Pages) -> Result<ExitCode, ExitCode> {
    let (_client, sessions) = connected()?;

    // The same resolve `spawn` checks against and the frame is drawn
    // from, run with no watchers -- so a live session arrives as `Quiet`
    // here and never as `Working`, which is why `listing::words` says
    // neither (`docs/07-cli.md`).
    let drawn = model::resolve(&sessions, &BTreeMap::new(), pages, tally::now_ms());
    let pads = listing::pads(&drawn, &sessions);

    // The same walk `attention` runs, and the same function: agents nest
    // so the innermost owner wins, and a dead row keeps its pid in
    // anchor's history while the OS reuses it. There is no second process
    // walk in this binary and there must not be.
    let ancestry = Ancestry::read();
    let me = std::process::id().cast_signed();
    let caller = match tally::attention::owning_session(&sessions, me, &ancestry) {
        None => Caller::Outside,
        Some(session) => match session.slot_label().and_then(|l| slot::parse(l, pages)) {
            Some(at) => Caller::At(at),
            // Running, and holding no pad: released with
            // `anchor label <id> tl.slot-`, or never claimed, or carrying
            // a label this grid cannot read. A real state, and the id is
            // what puts it back on the grid.
            None => Caller::Unplaced(&session.id),
        },
    };

    if json {
        println!("{}", listing::json(&pads, caller, pages));
    } else {
        print!("{}", listing::text(&pads, caller, pages));
    }
    Ok(ExitCode::SUCCESS)
}

/// Forget every ended row behind a pad, and close its view.
///
/// Lives here rather than on the grid because the usage says so — 341
/// jumps against zero purges — and because `anchor` has no `purge`
/// subcommand to defer to (anchor#14). Deleting the gesture without
/// replacing it would leave rows accumulating with no remedy anywhere
/// (`docs/05-interaction.md`).
fn purge(at: Slot, pages: &Pages) -> Result<ExitCode, ExitCode> {
    let address = address_of(at, pages)?;
    let (client, sessions) = connected()?;
    let rows = model::purgeable(&sessions, pages);
    let Some(ids) = rows.get(&at).filter(|ids| !ids.is_empty()) else {
        println!("nothing to purge on {address}");
        return Ok(ExitCode::SUCCESS);
    };
    let mut gone = 0;
    for id in ids {
        match client.purge(id) {
            Ok(()) => gone += 1,
            // Someone else forgot it first. That is the outcome we wanted.
            Err(tally::anchor::Error::Refused { ref code, .. }) if code == "not_found" => {}
            Err(e) => {
                eprintln!("tally: {e}");
                return Err(ExitCode::FAILURE);
            }
        }
        // Purge clears the pad, so it clears the view
        // (`docs/06-views.md`). Best-effort: a missing tmux is not a
        // reason to leave anchord holding the rows.
        let _closed = Tmux::default().close(id);
    }
    println!("purged {gone} on {address}");
    Ok(ExitCode::SUCCESS)
}

/// Stop a slot raising attention, or let it again.
///
/// The starvation hatch for `►` and the tempo: a prompt you are never
/// going to answer otherwise sits at the head of the queue forever and
/// drives the breathing up with it (`docs/05-interaction.md`).
fn mute(at: Slot, on: bool, pages: &Pages) -> Result<ExitCode, ExitCode> {
    let address = address_of(at, pages)?;
    let (client, sessions) = connected()?;
    let drawn = model::resolve(&sessions, &BTreeMap::new(), pages, tally::now_ms());
    let Some(held) = drawn.get(&at) else {
        eprintln!("tally: nothing is on {address}");
        return Err(ExitCode::FAILURE);
    };
    let mut set = BTreeMap::new();
    let remove: &[&str] = if on {
        let _m = set.insert(tally::anchor::LABEL_MUTE.to_owned(), "1".to_owned());
        &[]
    } else {
        &[tally::anchor::LABEL_MUTE]
    };
    match client.label(&held.session, &set, remove) {
        Ok(()) => {
            println!("{} {address}", if on { "muted" } else { "unmuted" });
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            eprintln!("tally: {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

/// The environment variable Claude Code sets for every hook it runs.
///
/// Measured against `claude` 2.1.236: present on `SessionStart` and on
/// `Stop`, and equal to the `session_id` in the JSON on stdin. The
/// variable rather than the JSON because this process runs on every turn
/// of every agent on the machine, and one `env::var` is cheaper than a
/// stdin slurp and a parse (`docs/06-views.md`).
const CLAUDE_SESSION_ENV: &str = "CLAUDE_CODE_SESSION_ID";

/// The conversation this hook is a turn of, if the caller is Claude Code
/// at all.
///
/// An empty value is `None`: the variable being set to nothing is how a
/// shell says it does not have one, and an empty `tl.claude` would put a
/// `claude --resume` with no argument on the dead line.
fn claude_session_id() -> Option<String> {
    std::env::var(CLAUDE_SESSION_ENV)
        .ok()
        .filter(|id| !id.is_empty())
}

/// Whether `tl.claude` needs writing, and with what.
///
/// Pure, and split out from the hook for that reason: the rule is *write
/// only what changed*, and the three cases it has to get right — not
/// Claude at all, a conversation already recorded, a conversation that
/// has been replaced by `/clear` — are three lines of test rather than
/// three agents and a stopwatch.
fn conversation_write(reported: Option<&str>, recorded: Option<&str>) -> Option<String> {
    reported
        .filter(|id| Some(*id) != recorded)
        .map(ToOwned::to_owned)
}

/// Set or remove `tl.attention` on whichever session we are inside, and
/// record which conversation it is running.
///
/// **The word, not the bit.** `Some` writes the value the rising hook
/// named and every one of them is an amber; which word, and why the grid
/// is not meant to tell them apart, is [`tally::anchor::Wants`].
///
/// **Not being under anchor is success**, and that is the whole shape of
/// this function. It is called from a Claude Code hook, which fires on
/// every turn of every agent on this machine; all but a handful are not on
/// the grid. An error there would put a line in the user's own agent, every
/// turn, for a surface that is not even watching it.
///
/// Nothing is ever written to stdout. stdout belongs to the agent's
/// transcript, and this is not something the agent said.
///
/// **Two facts, one hook.** `tl.claude` rides here rather than on a
/// `SessionStart` hook of its own because the expensive half of this
/// function is finding the session — a `ps -A` walk and an anchord
/// `list` — and a second hook would pay it again, per turn, for a value
/// that changes approximately never
/// (`docs/06-views.md`).
fn attention(wants: Option<tally::anchor::Wants>) -> Result<ExitCode, ExitCode> {
    // The one subcommand with a second reason to fail hard here: with
    // anchord down we cannot tell "not under anchor" -- which is success,
    // below -- from "could not look". See `docs/07-cli.md` for why the
    // hook line ends in `|| true` regardless.
    let (client, sessions) = connected()?;

    let ancestry = Ancestry::read();
    let me = std::process::id().cast_signed();
    let Some(session) = tally::attention::owning_session(&sessions, me, &ancestry) else {
        return Ok(ExitCode::SUCCESS);
    };

    let mut set = BTreeMap::new();
    // The value names which hook fired; presence is still the whole of
    // what raises the amber (#23).
    let remove: &[&str] = if let Some(wants) = wants {
        let _on = set.insert(
            tally::anchor::LABEL_ATTENTION.to_owned(),
            wants.as_str().to_owned(),
        );
        &[]
    } else {
        &[tally::anchor::LABEL_ATTENTION]
    };
    // Only when it changed. A `/clear` gives the same agent a new
    // conversation and must be followed; a quiet turn must not cost a
    // write that says what the label already says.
    if let Some(id) = conversation_write(claude_session_id().as_deref(), session.claude()) {
        let _conversation = set.insert(tally::anchor::LABEL_CLAUDE.to_owned(), id);
    }
    match client.label(&session.id, &set, remove) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        // The session ended between the list and the write. Nothing to
        // label, nothing wrong: a pad for a session that is gone is a pad
        // that renders its exit.
        Err(tally::anchor::Error::Refused { ref code, .. }) if code == "not_found" => {
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            eprintln!("tally: {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

/// Claim a pad for a session started in the current directory.
fn spawn(claim: &Claim, pages: &Pages) -> Result<ExitCode, ExitCode> {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("tally: cannot read the current directory: {e}");
            return Err(ExitCode::FAILURE);
        }
    };
    let command = claim
        .command
        .clone()
        .unwrap_or_else(|| cli::DEFAULT_COMMAND.to_owned());
    let (client, sessions) = connected()?;
    claim_pads(
        &client,
        &sessions,
        pages,
        claim,
        &Source {
            cwd,
            command,
            login_shell: true,
            from: None,
        },
    )
}

/// Claim a pad for a sibling of another pad.
///
/// `spawn` with one fact substituted — the directory comes from a pad
/// rather than from `$PWD` — which is why the whole of it below is
/// resolving that one fact and then calling the same function
/// (`docs/07-cli.md#tally-fork-src-dst`).
fn fork(fork: &Fork, pages: &Pages) -> Result<ExitCode, ExitCode> {
    let source = address_of(fork.from, pages)?;
    let (client, sessions) = connected()?;
    // The newest session that pad has ever held, dead rows included:
    // anchord keeps `cwd` on rows that have exited, which is the whole
    // reason forking a finished agent works at all.
    let Some(inherit) = model::inherit(&sessions, pages, fork.from) else {
        eprintln!("tally: nothing has ever run on {source}, so there is no directory to inherit");
        return Err(ExitCode::FAILURE);
    };
    claim_pads(
        &client,
        &sessions,
        pages,
        &fork.claim,
        &Source {
            cwd: PathBuf::from(inherit.cwd),
            // An explicit `--` beats the inheritance. The arguments are
            // never inherited either way: a pad running
            // `claude --resume <id>` is one conversation, and a second
            // agent must not be on it (`model::row_launch`).
            command: fork.claim.command.clone().unwrap_or(inherit.command),
            login_shell: inherit.login_shell,
            from: Some(source),
        },
    )
}

/// What a claim runs, once the half the parser could not know is resolved.
struct Source {
    cwd: PathBuf,
    command: String,
    login_shell: bool,
    /// The address this was forked from, for the line it prints.
    from: Option<String>,
}

/// Check every pad, then claim every pad.
///
/// **The check runs over all of them before the first create**, which is
/// the rule `--count` is worth having: a half-spawned fleet is a cleanup
/// job, and finding out on the third one is worse than retyping
/// (`docs/07-cli.md#--count-n`).
fn claim_pads(
    client: &Client,
    sessions: &[tally::anchor::Session],
    pages: &Pages,
    claim: &Claim,
    source: &Source,
) -> Result<ExitCode, ExitCode> {
    let Some(targets) = claim.targets(pages) else {
        eprintln!("tally: that address has no page name");
        return Err(ExitCode::FAILURE);
    };

    // The same predicate the grid previews with, run without watchers: a
    // pad holding something live refuses, a free one spawns. Literally
    // the same one -- it was a second copy of the `matches!` here, and a
    // grid that disagrees with the CLI is a grid that lies about what
    // pressing it does ([`SlotState::frees_the_pad`], #9).
    let drawn = model::resolve(sessions, &BTreeMap::new(), pages, tally::now_ms());
    let mut reclaiming = Vec::new();
    for (at, address) in &targets {
        if let Some(held) = drawn.get(at) {
            if !held.state.frees_the_pad() {
                eprintln!(
                    "tally: {address} is held by {} ({})",
                    held.session,
                    listing::words(held.state)
                );
                // The message used to name the holder and not the move, and
                // the documented move -- purge first -- is not reachable from
                // a shell at all (#9, anchor#14). Say what actually works.
                eprintln!(
                    "tally: if it is really gone: anchor label {} tl.slot- tl.attention-",
                    held.session
                );
                return Err(ExitCode::FAILURE);
            }
            // A row anchord lost to its own restart keeps its `tl.slot` and
            // its scrollback, and the newest session on a pad is the one the
            // grid draws -- so claiming the pad does not need the old row
            // removed, and must not destroy a log that is still worth
            // reading.
            reclaiming.push((address.clone(), held.session.clone(), held.state));
        }
    }
    for (address, session, state) in reclaiming {
        eprintln!(
            "tally: {address} was held by {session} ({}); reclaiming it",
            listing::words(state)
        );
    }

    // One pane and N sessions: picking one of them to open is a guess, so
    // a fleet opens nothing and every pad is one press from its view.
    let view = !claim.detach && claim.count == 1;
    for (at, address) in &targets {
        let mut labels = BTreeMap::new();
        let _slot = labels.insert(tally::anchor::LABEL_SLOT.to_owned(), address.clone());
        // No `tl.spec`: a spec label is a name looked up in `slots.toml`, and
        // one that nothing declares would be a label that lies. The cost is a
        // dead respawn chord on this pad (`docs/07-cli.md`).
        match client.create(&Spawn {
            command: &source.command,
            args: &claim.args,
            cwd: Some(&source.cwd),
            env: &BTreeMap::new(),
            login_shell: source.login_shell,
            labels: &labels,
        }) {
            Ok(id) => {
                if view {
                    place_or_report(&id, address, *at, &source.cwd);
                }
                match &source.from {
                    Some(from) => println!("forked {id} from {from} onto {address}"),
                    None => println!("spawned {id} on {address}"),
                }
            }
            // Stop and report. The ones already created are running in a
            // real repo and unwinding them would be a kill nobody asked
            // for, so the honest answer is the list above plus this line.
            Err(e) => {
                eprintln!("tally: {address}: {e}");
                return Err(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Put the view where the pad says.
///
/// Your shell is not touched; what `--detach` declines is the view, and
/// the story behind the name is on [`cli::Claim::detach`].
///
/// Placement is **not** fatal. The session exists by the time control
/// gets here, so a tmux that would not cooperate costs you a view that
/// the next press makes anyway.
fn place_or_report(id: &str, address: &str, at: Slot, cwd: &Path) {
    // The one caller that has the directory first-hand rather than back
    // from anchord, and the one that most wants it: this view is opened
    // the instant the session is created, from the shell you typed in.
    if let Err(e) = tally::jump::place(id, address, at.place(), cwd.to_str()) {
        eprintln!("tally: could not open a view for it: {e}");
        eprintln!("tally: press its pad to open it");
    }
}

#[cfg(test)]
mod tests {
    use super::conversation_write;

    /// The ordinary turn: the label says what the hook says, so the hook
    /// is a read and nothing else. This runs on every turn of every agent
    /// on the machine (`docs/06-views.md`).
    #[test]
    fn a_conversation_that_has_not_changed_is_not_written_again() {
        assert_eq!(conversation_write(Some("c15"), Some("c15")), None);
    }

    /// `/clear` keeps the agent and replaces the conversation. The label
    /// follows the conversation, because the label's whole job is to be
    /// the argument to `claude --resume`.
    #[test]
    fn a_cleared_conversation_replaces_the_one_on_the_label() {
        assert_eq!(
            conversation_write(Some("c99"), Some("c15")),
            Some("c99".to_owned())
        );
    }

    #[test]
    fn a_first_turn_records_the_conversation() {
        assert_eq!(
            conversation_write(Some("c15"), None),
            Some("c15".to_owned())
        );
    }

    /// Not Claude Code at all — a shell on a pad. Nothing reported means
    /// nothing written, and in particular **not** a removal: tally does
    /// not get to forget a conversation because something else ran a hook.
    #[test]
    fn nothing_reported_writes_nothing() {
        assert_eq!(conversation_write(None, Some("c15")), None);
        assert_eq!(conversation_write(None, None), None);
    }
}
