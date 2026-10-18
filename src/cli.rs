//! Argument parsing for the `tally` command.
//!
//! Lives in the library rather than in the binary so the rules can be
//! tested without spawning a process. The rule worth testing is the one
//! that says **refused, not clamped**: an address outside the grid was
//! written by something that disagrees with us about the address space,
//! and putting it somewhere arbitrary is worse than rejecting it
//! (`docs/02-model.md#limits`).

use std::fmt;

use crate::slot::{self, Pages, Slot, PAGES};

/// What a bare `tally spawn` runs. The thing this grid is for.
pub const DEFAULT_COMMAND: &str = "claude";

/// The one-screen help, printed for `help` and for every usage error.
pub const USAGE: &str = "\
usage: tally ls    [--json]
       tally spawn [--detach] [--count N] <addr> [-- command [args...]]
       tally fork  [--detach] [--count N] <src> <dst> [-- command [args...]]
       tally purge     <addr>
       tally mute      <addr> [--off]
       tally attention <on | off>

An <addr> is row:col, or page:row:col.

ls      Print the grid, and mark the pad you are on.

  tally ls                             the grid as text
  tally ls --json                      the same, for a script

       1  2  3  4  5  6  7  8
  1:1  #  #  #  @  .  .  .  .
  1:3  !  .  .  .  .  .  .  .

  @ you   # running   ? waiting   ! failed   x lost   . free

A row label and a column heading joined is the address: row 1:1, column
5, is 1:1:5. Under the picture is a line per occupied pad with its
session id, its state and its directory.

It takes no address, and it finds you the way attention does -- by
walking the process tree to whichever session this process is inside.
Not being under anchor prints the grid without a @ and is not an error.
Rows holding nothing are not shown.

It does not pick a pad for you. Read the grid and type the address.

spawn   Claim a pad for a session started in the current directory.

  tally spawn 1:1                      claude, here, on page 1
  tally spawn 3:4:4                    claude, here, on page 3
  tally spawn work:3:4 -- cargo test   a different command
  tally spawn --detach 1:1             spawn it, open no view

Your shell is not handed over. The view opens on tally's own tmux
server and this command returns; --detach opens no view at all, and the
pad still lights.

The directory is always the one you are standing in, which is the whole
point; there is no --cwd. Two-part addresses are row:col on the first
page. Row 1 is the top, column 1 is the left.

fork    Another one of those. Same as spawn, except the directory comes
        from a pad instead of from here.

  tally fork 1:1 1:2                   a fresh agent, same folder
  tally fork 1:1 2:1 -- cargo watch    same folder, your command

It inherits cwd, argv[0] and the login shell from the newest session the
source pad has ever held, dead rows included. It does not inherit the
arguments: a pad running `claude --resume <id>` is one conversation, and
a second agent must not be on it.

--count N   Claim N consecutive pads rightwards, in that row.

  tally spawn 2:1 --count 4 -- claude -p \"port the parser\"

The goal is argv, not a flag. N is 1-8, it refuses rather than running
past column 8 or wrapping, and it refuses before creating anything if
any target is taken. N above 1 opens no views: N sessions, N pads
lit, nothing on screen.

attention   Say whether the calling session is waiting on a human.

  tally attention on                   it wants you    (sets tl.attention)
  tally attention off                  it does not     (removes it)

For a Claude Code hook. It takes no address: the session is whichever one
this process is running inside, found by walking the process tree. Not
being under anchor is success and prints nothing.

purge   Forget the ended rows behind a pad. It does not kill anything.

  tally purge 1:3:1                    forget what has exited here
  tally purge work:3:4                 same, on a named page

Every exited or lost session that claims the pad goes, rows and logs
together, and their views close. A session that is still running is never
touched: if nothing has ended behind the pad it prints \"nothing to purge\"
and exits 0, and whatever was running keeps running.

To free a pad that holds a live session, release it, end it, claim it:

  anchor label <id> tl.slot- tl.attention-    let go of the pad
  anchor kill <id>                            end the session
  tally spawn 1:3:1                           the pad is free

The id is in the message spawn refuses with.

mute    Stop a pad raising attention, or let it again.

  tally mute 1:3:1                     stop it asking for you
  tally mute 1:3:1 --off               let it ask again

For a prompt you are never going to answer, which would otherwise sit at
the head of the queue and keep the tempo up. Only attention is muted: the
pad still shows working or quiet, and the session is not touched. It
sticks to the session on the pad, not to the pad, so a fresh spawn there
starts unmuted. It refuses when nothing is on the pad. It takes a flag
rather than toggling because a shell cannot see the colour.
";

/// What the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Print [`USAGE`] and stop.
    Help,
    /// Print the grid, marking the calling session's pad. `true` for JSON.
    ///
    /// No address, for the same reason [`Self::Attention`] has none: the
    /// interesting pad is the one this process is inside, and nothing in
    /// a hook's or an agent's environment knows which that is
    /// (`docs/07-cli.md`).
    Ls(bool),
    /// Claim a pad.
    Spawn(Claim),
    /// Claim a pad for a sibling of another pad.
    ///
    /// The same request as [`Self::Spawn`] with one fact substituted: the
    /// directory comes from the source pad rather than from `$PWD`. The
    /// substitution cannot happen here — it needs anchord's session list —
    /// so the parser carries the source address and the binary resolves it
    /// (`docs/07-cli.md`).
    Fork(Fork),
    /// Declare whether the calling session is waiting on a human.
    ///
    /// `true` sets `tl.attention`, `false` removes it. There is no address
    /// because there is nobody to type one: the only caller is a hook
    /// inside the session it is talking about (`docs/04-signals.md`).
    Attention(bool),
    /// `tally purge <addr>` — forget every ended row behind a pad.
    ///
    /// It has an address because the caller is you, not a hook, and
    /// because `anchor` has no `purge` subcommand to defer to
    /// (anchor#14). Moved here from the grid when the right column became
    /// launchers (`docs/05-interaction.md`).
    Purge(Slot),
    /// `tally mute <addr> [--off]` — stop a slot raising attention.
    ///
    /// Rare enough to live in a shell, and necessary enough not to delete:
    /// it is the starvation hatch for `►` and for the tempo
    /// (`docs/05-interaction.md`, `docs/05-interaction.md`).
    Mute(Slot, bool),
}

/// A validated `tally spawn`: a real address and a command, if one was typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The first pad being claimed. Its page is known to be declared.
    pub at: Slot,
    /// `argv[0]`, if `--` gave one.
    ///
    /// `None` rather than [`DEFAULT_COMMAND`] because the two callers
    /// default differently: a spawn runs `claude`, a fork runs whatever
    /// the source pad ran. Resolving it here would make the fork's
    /// inheritance unreachable for the common case of not typing `--`.
    pub command: Option<String>,
    /// `argv[1..]`.
    pub args: Vec<String>,
    /// Spawn it and open no view for it.
    ///
    /// It used to mean *do not hand this pane over*, and that is no
    /// longer a thing that can happen: views live on tally's own tmux
    /// server and your shell is never handed over
    /// (`docs/06-views.md`). What is left to decline is the
    /// **view** — the session runs, the pad lights, and nothing opens
    /// until you press it.
    ///
    /// `--count N` above 1 implies it, because one shell cannot be shown
    /// N views and picking one of them is a guess.
    pub detach: bool,
    /// How many consecutive pads, rightwards in [`Self::at`]'s row.
    ///
    /// Always `1..=8`, and always inside the row: a count that would run
    /// past column 8 is refused by the parser rather than clamped or
    /// wrapped, which is the same rule an address gets
    /// (`docs/02-model.md#limits`).
    pub count: u8,
}

/// A validated `tally fork`: where to inherit from, and the claim it feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fork {
    /// The pad to inherit `cwd`, `argv[0]` and the login shell from.
    ///
    /// Not checked for being occupied, and deliberately: you fork from a
    /// pad whose agent has *ended* as readily as from a live one, because
    /// anchord keeps the directory on dead rows too
    /// (`docs/05-interaction.md`).
    pub from: Slot,
    /// Where it lands, and with what. Identical to a spawn's.
    pub claim: Claim,
}

/// Why the arguments were refused. Both variants are the user's typing,
/// so both exit `2` and both print [`USAGE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The shape of the command line is wrong.
    Usage(String),
    /// The address is not one this grid has.
    Address(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(m) | Self::Address(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

/// Parse the arguments after the program name.
///
/// # Errors
/// [`Error`] for anything that is not a complete, valid request. Nothing
/// here touches the filesystem or anchord: this decides only whether the
/// words make sense.
pub fn parse(args: impl IntoIterator<Item = String>, pages: &Pages) -> Result<Request, Error> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err(Error::Usage("no subcommand".to_owned()));
    };
    match first.as_str() {
        "help" | "-h" | "--help" => Ok(Request::Help),
        "ls" => ls(args).map(Request::Ls),
        "spawn" => claim(args, pages).map(Request::Spawn),
        "fork" => fork(args, pages).map(Request::Fork),
        "attention" => attention(args).map(Request::Attention),
        "purge" => one_address(args, pages).map(Request::Purge),
        "mute" => mute(args, pages),
        other => Err(Error::Usage(format!("unknown subcommand {other:?}"))),
    }
}

/// `on` or `off`, and nothing else.
///
/// Deliberately not accepting `1`/`0`/`true`/`yes`: the caller is a line in
/// a settings file that somebody wrote once, so there is no ergonomic case
/// for aliases, and every alias is another spelling to recognise when the
/// pad is not lighting up.
fn attention(mut args: impl Iterator<Item = String>) -> Result<bool, Error> {
    let Some(word) = args.next() else {
        return Err(Error::Usage("attention needs `on` or `off`".to_owned()));
    };
    let on = match word.as_str() {
        "on" => true,
        "off" => false,
        other => {
            return Err(Error::Usage(format!(
                "attention takes `on` or `off`, not {other:?}"
            )))
        }
    };
    if let Some(extra) = args.next() {
        return Err(Error::Usage(format!("unexpected {extra:?}")));
    }
    Ok(on)
}

/// `ls [--json]`, and nothing else.
///
/// It refuses an address rather than ignoring one. `tally ls 1:1` is a
/// reader asking to filter eight lines of output, and silently printing
/// the whole grid instead would be answering a question they did not ask
/// (`docs/07-cli.md`).
fn ls(mut args: impl Iterator<Item = String>) -> Result<bool, Error> {
    let json = match args.next().as_deref() {
        None => false,
        Some("--json") => true,
        Some(other) => return Err(Error::Usage(format!("ls takes --json, not {other:?}"))),
    };
    if let Some(extra) = args.next() {
        return Err(Error::Usage(format!("unexpected {extra:?}")));
    }
    Ok(json)
}

/// An address and nothing else.
fn one_address(mut args: impl Iterator<Item = String>, pages: &Pages) -> Result<Slot, Error> {
    let Some(text) = args.next() else {
        return Err(Error::Usage("an address is required".to_owned()));
    };
    let at = address(&text, pages)?;
    if let Some(extra) = args.next() {
        return Err(Error::Usage(format!("unexpected {extra:?}")));
    }
    Ok(at)
}

/// `mute <addr> [--off]`.
///
/// A flag rather than a toggle, because a shell cannot see the pad. The
/// grid's gesture toggled because you were looking at the colour; from a
/// script you have to say which way you meant it.
fn mute(mut args: impl Iterator<Item = String>, pages: &Pages) -> Result<Request, Error> {
    let Some(text) = args.next() else {
        return Err(Error::Usage("mute needs an address".to_owned()));
    };
    let at = address(&text, pages)?;
    let on = match args.next().as_deref() {
        None => true,
        Some("--off") => false,
        Some(other) => return Err(Error::Usage(format!("unexpected {other:?}"))),
    };
    if let Some(extra) = args.next() {
        return Err(Error::Usage(format!("unexpected {extra:?}")));
    }
    Ok(Request::Mute(at, on))
}

/// The most pads one `--count` can claim: a row is eight wide.
const MAX_COUNT: u8 = 8;

/// `spawn [--detach] [--count N] <addr> [-- command [args...]]`.
fn claim(args: impl Iterator<Item = String>, pages: &Pages) -> Result<Claim, Error> {
    Ok(tail(args, pages, false, "spawn")?.claim)
}

/// `fork [--detach] [--count N] <src> <dst> [-- command [args...]]`.
///
/// Two addresses and otherwise identical, which is why it shares
/// [`tail`]: every flag rule here was argued for `spawn` and none of the
/// arguments change when the directory comes from a pad.
fn fork(args: impl Iterator<Item = String>, pages: &Pages) -> Result<Fork, Error> {
    let tail = tail(args, pages, true, "fork")?;
    // `tail` guarantees the source is present when it was asked for, so
    // the only way to reach this with `None` is a caller passing `false`
    // above — which is the other subcommand.
    let from = tail
        .source
        .ok_or_else(|| Error::Usage(format!("fork needs {}", addresses(true))))?;
    Ok(Fork {
        from,
        claim: tail.claim,
    })
}

/// Everything `spawn` and `fork` share: flags, the addresses, then `--`.
struct Tail {
    /// The pad to inherit from, when the subcommand takes one.
    source: Option<Slot>,
    /// Where it lands, and with what.
    claim: Claim,
}

/// Parse `[flags] <addr> [<addr>] [-- command [args...]]`.
///
/// `sourced` is the only thing that differs between the two subcommands:
/// a fork names the pad it comes from first, and a spawn comes from the
/// directory you are standing in. `what` exists so the refusal names the
/// thing the user typed rather than the thing this function is.
fn tail(
    args: impl Iterator<Item = String>,
    pages: &Pages,
    sourced: bool,
    what: &str,
) -> Result<Tail, Error> {
    let want = usize::from(sourced) + 1;
    let mut args = args.into_iter();
    let mut detach = false;
    let mut count: Option<u8> = None;
    let mut text: Vec<String> = Vec::new();
    let mut command: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();

    // Everything after `--` is the command. Anything *before* it that is
    // not an address or a flag we have would be silently ignored
    // otherwise, and a spawn that runs something other than what was typed
    // is the one outcome this parser exists to prevent.
    while let Some(token) = args.next() {
        match token.as_str() {
            "--" => {
                if text.len() < want {
                    return Err(Error::Usage(format!(
                        "{what} needs {} before `--`",
                        addresses(sourced)
                    )));
                }
                let Some(first) = args.next() else {
                    return Err(Error::Usage("`--` with nothing after it".to_owned()));
                };
                command = Some(first);
                rest = args.collect();
                break;
            }
            "--detach" => {
                if detach {
                    return Err(Error::Usage("--detach given twice".to_owned()));
                }
                detach = true;
            }
            "--count" => {
                if count.is_some() {
                    return Err(Error::Usage("--count given twice".to_owned()));
                }
                let Some(n) = args.next() else {
                    return Err(Error::Usage("--count needs a number".to_owned()));
                };
                count = Some(match n.parse::<u8>() {
                    Ok(n) if (1..=MAX_COUNT).contains(&n) => n,
                    _ => return Err(Error::Usage(format!("--count {n:?} is not 1-{MAX_COUNT}"))),
                });
            }
            flag if flag.starts_with('-') => {
                return Err(Error::Usage(format!("unknown flag {flag:?}")));
            }
            _ if text.len() == want => {
                return Err(Error::Usage(format!(
                    "unexpected {token:?}; a command goes after `--`"
                )));
            }
            _ => text.push(token),
        }
    }

    let mut typed = text.iter();
    let mut next = || match typed.next() {
        Some(t) => address(t, pages),
        None => Err(Error::Usage(format!("{what} needs {}", addresses(sourced)))),
    };
    let first = next()?;
    // A fork names its source first and its destination second, so the
    // pad being claimed is the last address either subcommand takes.
    let (source, at) = if sourced {
        (Some(first), next()?)
    } else {
        (None, first)
    };
    let count = count.unwrap_or(1);

    // Refused rather than clamped, and never wrapped onto the next row: a
    // fleet that silently lands somewhere other than where it was aimed is
    // worse than retyping (`docs/07-cli.md#--count-n`).
    if u16::from(at.col) + u16::from(count) - 1 > u16::from(MAX_COUNT) {
        return Err(Error::Address(format!(
            "--count {count} from column {} runs past column {MAX_COUNT}",
            at.col
        )));
    }

    Ok(Tail {
        source,
        claim: Claim {
            at,
            command,
            args: rest,
            detach,
            count,
        },
    })
}

/// "an address" or "two addresses", for a refusal that reads like English.
fn addresses(sourced: bool) -> &'static str {
    if sourced {
        "a source and a destination address"
    } else {
        "an address"
    }
}

/// Parse an address a human typed: `row:col`, or `page:row:col`.
///
/// Two parts means the first page, because most people live on one and
/// typing its name every time is the friction this command exists to
/// remove.
///
/// **The three-part form is [`crate::slot::parse`]'s grammar and is
/// answered by it**, so what this command accepts and what a `tl.slot`
/// label means cannot drift apart. The walk below runs only for an
/// address that grammar rejected, and only to say *which part* of it was
/// wrong — a human typing into a shell gets a sentence, where a label
/// read off a session gets `None` and is left off the grid
/// (`docs/08-daemon.md`).
///
/// # Errors
/// [`Error::Address`] for the wrong number of parts, a coordinate outside
/// `1..=8`, or a page no `slots.toml` declares. Never clamped.
pub fn address(text: &str, pages: &Pages) -> Result<Slot, Error> {
    if let Some(at) = slot::parse(text, pages) {
        return Ok(at);
    }
    let parts: Vec<&str> = text.split(':').collect();
    let (name, row, col) = match parts.as_slice() {
        [row, col] => (None, *row, *col),
        [page, row, col] => (Some(*page), *row, *col),
        _ => {
            return Err(Error::Address(format!(
                "{text:?} is not row:col or page:row:col"
            )))
        }
    };
    let page = match name {
        Some(name) => pages.index_of(name).ok_or_else(|| {
            Error::Address(format!(
                "no page named {name:?}; declared: {}",
                declared(pages)
            ))
        })?,
        // A config declaring no pages at all would make every address
        // unwritable, because the label needs a name. Say so here rather
        // than spawning something no pad can show.
        None if pages.is_empty() => {
            return Err(Error::Address(
                "slots.toml declares no pages, so there is nowhere to spawn".to_owned(),
            ))
        }
        None => 1,
    };
    Ok(Slot {
        page,
        row: coordinate(row, "row")?,
        col: coordinate(col, "col")?,
    })
}

fn coordinate(text: &str, what: &str) -> Result<u8, Error> {
    match text.parse::<u8>() {
        Ok(n) if (1..=8).contains(&n) => Ok(n),
        _ => Err(Error::Address(format!("{what} {text:?} is not 1-8"))),
    }
}

fn declared(pages: &Pages) -> String {
    (1..=PAGES)
        .filter_map(|n| pages.name_of(n))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named() -> Pages {
        Pages::new(["work".to_owned(), "infra".to_owned()])
    }

    fn spawn(args: &[&str], pages: &Pages) -> Result<Claim, Error> {
        match parse(args.iter().map(|s| (*s).to_owned()), pages)? {
            Request::Spawn(claim) => Ok(claim),
            other => panic!("asked to spawn, got {other:?}"),
        }
    }

    fn ls_of(args: &[&str]) -> Result<bool, Error> {
        match parse(args.iter().map(|s| (*s).to_owned()), &named())? {
            Request::Ls(json) => Ok(json),
            other => panic!("asked to list, got {other:?}"),
        }
    }

    /// No address and one flag. `ls` is the only subcommand that reads
    /// the grid without naming part of it.
    #[test]
    fn ls_takes_nothing_or_json() {
        assert_eq!(ls_of(&["ls"]), Ok(false));
        assert_eq!(ls_of(&["ls", "--json"]), Ok(true));
    }

    /// Refused rather than ignored. `tally ls 1:1` is a reader asking to
    /// filter the output, and printing the whole grid anyway would be
    /// answering a question they did not ask.
    #[test]
    fn ls_refuses_an_address_rather_than_ignoring_it() {
        let refused = ls_of(&["ls", "1:1"]).expect_err("an address is not a flag");
        assert!(matches!(refused, Error::Usage(_)), "{refused:?}");
        let _twice = ls_of(&["ls", "--json", "--json"]).unwrap_err();
        let _unknown = ls_of(&["ls", "--long"]).unwrap_err();
        let _trailing = ls_of(&["ls", "--json", "extra"]).unwrap_err();
    }

    fn attention_of(args: &[&str]) -> Result<bool, Error> {
        match parse(args.iter().map(|s| (*s).to_owned()), &named())? {
            Request::Attention(on) => Ok(on),
            other => panic!("asked for attention, got {other:?}"),
        }
    }

    /// Flags are reachable from either side of the address — a shell user
    /// types them where it occurs to them — and a command after `--` is
    /// the command; nothing before it is.
    #[test]
    fn an_address_may_be_followed_by_a_command() {
        let pages = named();
        let plain = spawn(&["spawn", "1:1"], &pages).expect("plain");
        assert_eq!(plain.command, None);
        assert!(!plain.detach);
        assert_eq!(plain.count, 1);
        assert!(
            spawn(&["spawn", "--detach", "1:1"], &pages)
                .expect("before")
                .detach
        );
        assert!(
            spawn(&["spawn", "1:1", "--detach"], &pages)
                .expect("after")
                .detach
        );
        assert_eq!(
            spawn(&["spawn", "1:1", "--count", "4"], &pages)
                .expect("count")
                .count,
            4
        );
        let with_cmd =
            spawn(&["spawn", "1:1", "--detach", "--", "ls", "-l"], &pages).expect("with a command");
        assert!(with_cmd.detach);
        assert_eq!(with_cmd.command.as_deref(), Some("ls"));
        assert_eq!(with_cmd.args, ["-l"]);
    }

    fn fork_of(args: &[&str], pages: &Pages) -> Result<Fork, Error> {
        match parse(args.iter().map(|s| (*s).to_owned()), pages)? {
            Request::Fork(fork) => Ok(fork),
            other => panic!("asked to fork, got {other:?}"),
        }
    }

    /// Two addresses, in the order you say them: from here, onto there.
    #[test]
    fn fork_takes_a_source_and_a_destination() {
        let pages = named();
        let fork = fork_of(&["fork", "1:1", "infra:3:4"], &pages).expect("valid");
        assert_eq!(
            fork.from,
            Slot {
                page: 1,
                row: 1,
                col: 1
            }
        );
        assert_eq!(
            fork.claim.at,
            Slot {
                page: 2,
                row: 3,
                col: 4
            }
        );
        // Left unresolved on purpose: the binary falls back to the source
        // pad's command, which is the whole point of the subcommand, and
        // it cannot do that if the parser has already said `claude`.
        assert_eq!(fork.claim.command, None);
    }

    /// Everything `spawn` learned, `fork` has too: one parser, two
    /// subcommands, and no second set of flag rules to keep in step.
    #[test]
    fn fork_takes_the_same_flags_and_command() {
        let pages = named();
        let fork = fork_of(
            &["fork", "--count", "2", "1:1", "1:3", "--", "cargo", "test"],
            &pages,
        )
        .expect("valid");
        assert_eq!(fork.claim.count, 2);
        assert_eq!(fork.claim.command.as_deref(), Some("cargo"));
        assert_eq!(fork.claim.args, ["test"]);
    }

    /// One address is not two, and three is not two either.
    #[test]
    fn fork_refuses_one_address_or_three() {
        let pages = named();
        for args in [
            &["fork"][..],
            &["fork", "1:1"],
            &["fork", "1:1", "--", "ls"],
            &["fork", "1:1", "1:2", "1:3"],
            &["fork", "1:1", "nope:1:1"],
        ] {
            assert!(fork_of(args, &pages).is_err(), "{args:?} was accepted");
        }
    }

    /// Refused, not clamped and never wrapped onto the next row: a fleet
    /// that lands somewhere other than where it was aimed is worse than
    /// retyping.
    #[test]
    fn a_count_that_leaves_the_row_is_refused() {
        let pages = named();
        assert!(matches!(
            spawn(&["spawn", "1:7", "--count", "3"], &pages),
            Err(Error::Address(_))
        ));
        // It is the *destination* that is checked, not the source.
        assert!(matches!(
            fork_of(&["fork", "1:1", "1:8", "--count", "2"], &pages),
            Err(Error::Address(_))
        ));
        // Exactly reaching column 8 is fine; one past it is not.
        assert_eq!(
            spawn(&["spawn", "1:5", "--count", "4"], &pages)
                .expect("to the edge")
                .count,
            4
        );
    }

    /// A count outside 1-8 is typing, not an address, and says so.
    #[test]
    fn a_count_outside_the_row_width_is_a_usage_error() {
        let pages = named();
        for args in [
            &["spawn", "1:1", "--count", "0"][..],
            &["spawn", "1:1", "--count", "9"],
            &["spawn", "1:1", "--count", "x"],
            &["spawn", "1:1", "--count"],
            &["spawn", "1:1", "--count", "2", "--count", "2"],
        ] {
            assert!(
                matches!(spawn(args, &pages), Err(Error::Usage(_))),
                "{args:?} was accepted"
            );
        }
    }

    /// A flag we do not have must not be silently dropped: it would spawn
    /// something other than what was typed, which is the one outcome this
    /// parser exists to prevent.
    #[test]
    fn an_unknown_flag_is_refused_rather_than_ignored() {
        let pages = named();
        for args in [
            &["spawn", "--cwd", "1:1"][..],
            &["spawn", "1:1", "-x"],
            &["spawn", "--detach", "--detach", "1:1"],
        ] {
            assert!(
                matches!(spawn(args, &pages), Err(Error::Usage(_))),
                "{args:?} was accepted"
            );
        }
    }

    /// A flag is not an address. The old parser took the first token it
    /// saw, so a bare flag would have tried to parse as one.
    #[test]
    fn a_flag_alone_is_not_an_address() {
        assert!(matches!(
            spawn(&["spawn", "--detach"], &named()),
            Err(Error::Usage(_))
        ));
    }

    fn one(args: &[&str]) -> Result<Request, Error> {
        parse(args.iter().map(|s| (*s).to_owned()), &named())
    }

    /// Both moved off the grid when the right column became launchers,
    /// and both had to keep existing: `anchor` has no purge of its own,
    /// and mute is the starvation hatch for `►` and the tempo.
    #[test]
    fn purge_and_mute_take_an_address() {
        let at = Slot {
            page: 2,
            row: 3,
            col: 4,
        };
        assert_eq!(one(&["purge", "infra:3:4"]), Ok(Request::Purge(at)));
        assert_eq!(one(&["mute", "infra:3:4"]), Ok(Request::Mute(at, true)));
        assert_eq!(
            one(&["mute", "infra:3:4", "--off"]),
            Ok(Request::Mute(at, false))
        );
    }

    /// A shell cannot see the pad, so mute says which way it meant rather
    /// than toggling blind.
    #[test]
    fn purge_and_mute_refuse_what_they_cannot_act_on() {
        for args in [
            &["purge"][..],
            &["purge", "1:1", "extra"],
            &["purge", "nope:1:1"],
            &["mute"],
            &["mute", "1:1", "--on"],
            &["mute", "1:1", "--off", "extra"],
        ] {
            assert!(one(args).is_err(), "{args:?} was accepted");
        }
    }

    #[test]
    fn attention_takes_on_and_off() {
        assert_eq!(attention_of(&["attention", "on"]), Ok(true));
        assert_eq!(attention_of(&["attention", "off"]), Ok(false));
    }

    /// No aliases, by decision. The caller is one line in a settings file,
    /// and every spelling accepted is another one to recognise later when
    /// the pad is not lighting up.
    #[test]
    fn attention_refuses_anything_else() {
        for args in [
            &["attention"][..],
            &["attention", "1"],
            &["attention", "true"],
            &["attention", "yes"],
            &["attention", "on", "extra"],
        ] {
            assert!(
                matches!(
                    parse(args.iter().map(|s| (*s).to_owned()), &named()),
                    Err(Error::Usage(_))
                ),
                "{args:?} should be a usage error"
            );
        }
    }

    /// The headline case: a short address, and the command you did not type.
    #[test]
    fn two_parts_is_the_first_page_and_claude() {
        let claim = spawn(&["spawn", "1:1"], &named()).expect("valid");
        assert_eq!(
            claim.at,
            Slot {
                page: 1,
                row: 1,
                col: 1
            }
        );
        assert_eq!(claim.command, None, "a bare spawn defaults downstream");
        assert!(claim.args.is_empty());
    }

    /// The label a spawn writes has to be readable back, and that is only
    /// true if the page it names was declared. A break here is a session
    /// that exists and a pad that never lights.
    #[test]
    fn an_address_round_trips_to_a_label() {
        let pages = named();
        let claim = spawn(&["spawn", "infra:8:8"], &pages).expect("valid");
        let label = claim.at.label(&pages).expect("page is declared");
        assert_eq!(label, "infra:8:8");
        assert_eq!(slot::parse(&label, &pages), Some(claim.at));
    }

    /// With no `slots.toml` the pages are named "1" to "8", so a numeric
    /// page in a three-part address is a name lookup like any other.
    #[test]
    fn default_pages_are_addressable_by_number() {
        let claim = spawn(&["spawn", "3:4:5"], &Pages::default()).expect("valid");
        assert_eq!(
            claim.at,
            Slot {
                page: 3,
                row: 4,
                col: 5
            }
        );
    }

    /// Refused, not clamped -- the same rule a label follows.
    #[test]
    fn an_address_off_the_grid_is_refused() {
        for bad in ["0:1", "9:1", "1:9", "1:0", "a:b", "1", "1:2:3:4", "", "1:"] {
            let got = spawn(&["spawn", bad], &Pages::default());
            assert!(
                matches!(got, Err(Error::Address(_))),
                "{bad:?} should be refused, got {got:?}"
            );
        }
    }

    #[test]
    fn an_undeclared_page_is_refused_and_says_what_exists() {
        let Err(Error::Address(message)) = spawn(&["spawn", "nope:1:1"], &named()) else {
            panic!("an undeclared page must be refused");
        };
        assert!(message.contains("work, infra"), "{message}");
    }

    #[test]
    fn a_command_after_the_separator_wins() {
        let claim = spawn(
            &["spawn", "1:1", "--", "cargo", "watch", "-x", "test"],
            &named(),
        )
        .expect("valid");
        assert_eq!(claim.command.as_deref(), Some("cargo"));
        assert_eq!(claim.args, ["watch", "-x", "test"]);
    }

    /// An empty argv is a config error, not a spawn of the empty string.
    #[test]
    fn a_separator_with_nothing_after_it_is_refused() {
        assert!(matches!(
            spawn(&["spawn", "1:1", "--"], &named()),
            Err(Error::Usage(_))
        ));
    }

    /// Silently ignoring a flag we do not have would spawn something other
    /// than what was typed.
    #[test]
    fn a_word_before_the_separator_is_refused() {
        assert!(matches!(
            spawn(&["spawn", "1:1", "--cwd", "/tmp"], &named()),
            Err(Error::Usage(_))
        ));
    }

    #[test]
    fn the_address_is_required() {
        for args in [vec!["spawn"], vec!["spawn", "--", "claude"]] {
            assert!(
                matches!(spawn(&args, &named()), Err(Error::Usage(_))),
                "{args:?} should be refused"
            );
        }
    }

    #[test]
    fn help_and_nothing_are_told_apart() {
        let pages = Pages::default();
        for word in ["help", "-h", "--help"] {
            let got = parse([word.to_owned()], &pages);
            assert_eq!(got, Ok(Request::Help), "{word}");
        }
        assert!(matches!(parse([], &pages), Err(Error::Usage(_))));
        // `ls` stood here until it became a subcommand
        // (`docs/07-cli.md`). `list` is the word the agent in
        // that document typed at `anchor` and got refused for, which makes
        // it the honest example of one we do not have.
        assert!(matches!(
            parse(["list".to_owned()], &pages),
            Err(Error::Usage(_))
        ));
    }

    /// A config with no pages has nowhere to put anything, and saying so
    /// beats writing a label no pad can show.
    #[test]
    fn no_declared_pages_means_no_address() {
        let none = Pages::new([]);
        assert!(matches!(
            spawn(&["spawn", "1:1"], &none),
            Err(Error::Address(_))
        ));
    }

    /// What a human may type and what a `tl.slot` label may carry are one
    /// grammar for the three-part form, because this function asks
    /// `slot::parse` first (`docs/08-daemon.md`). The
    /// two-part form is this command's own convenience and deliberately
    /// never becomes a label.
    #[test]
    fn a_typed_address_and_a_label_are_the_same_grammar() {
        let pages = named();
        for text in ["work:3:5", "work:1:1", "work:8:8"] {
            assert_eq!(
                address(text, &pages).ok(),
                slot::parse(text, &pages),
                "{text}"
            );
        }
        // And every rejection is a rejection in both, with a sentence
        // here and a `None` there.
        for bad in ["work:9:1", "work:0:1", "nope:1:1", "work:1:2:3"] {
            assert!(address(bad, &pages).is_err(), "{bad}");
            assert!(slot::parse(bad, &pages).is_none(), "{bad}");
        }
        // The two-part form is ours alone.
        address("3:5", &pages).expect("two parts is this command's own form");
        assert!(slot::parse("3:5", &pages).is_none());
    }
}
