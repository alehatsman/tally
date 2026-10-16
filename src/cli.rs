//! Argument parsing for the `tally` command.
//!
//! Lives in the library rather than in the binary so the rules can be
//! tested without spawning a process. The rule worth testing is the one
//! that says **refused, not clamped**: an address outside the grid was
//! written by something that disagrees with us about the address space,
//! and putting it somewhere arbitrary is worse than rejecting it
//! (`docs/03-model.md#limits`).

use std::fmt;

use crate::slot::{Pages, Slot, PAGES};

/// What a bare `tally spawn` runs. The thing this grid is for.
pub const DEFAULT_COMMAND: &str = "claude";

/// The one-screen help, printed for `help` and for every usage error.
pub const USAGE: &str = "\
usage: tally spawn [--detach] <row:col | page:row:col> [-- command [args...]]
       tally purge     <row:col | page:row:col>
       tally mute      <row:col | page:row:col> [--off]
       tally attention <on | off>

spawn   Claim a pad for a session started in the current directory, and
        hand this pane over to it.

  tally spawn 1:1                      claude, here, on page 1
  tally spawn 3:4:4                    claude, here, on page 3
  tally spawn work:3:4 -- cargo test   a different command
  tally spawn --detach 1:1             spawn it and come back

Inside tmux this pane becomes the view: the shell is replaced by the
attach, and the pad focuses this pane. --detach keeps the old behaviour
and is what a script wants. Outside tmux there is nothing to adopt, so
it prints and returns either way.

The directory is always the one you are standing in, which is the whole
point; there is no --cwd. Two-part addresses are row:col on the first
page. Row 1 is the top, column 1 is the left.

attention   Say whether the calling session is waiting on a human.

  tally attention on                   it wants you    (sets tl.attention)
  tally attention off                  it does not     (removes it)

For a Claude Code hook. It takes no address: the session is whichever one
this process is running inside, found by walking the process tree. Not
being under anchor is success and prints nothing.
";

/// What the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Print [`USAGE`] and stop.
    Help,
    /// Claim a pad.
    Spawn(Claim),
    /// Declare whether the calling session is waiting on a human.
    ///
    /// `true` sets `tl.attention`, `false` removes it. There is no address
    /// because there is nobody to type one: the only caller is a hook
    /// inside the session it is talking about (`docs/14-attention.md`).
    Attention(bool),
    /// `tally purge <addr>` — forget every ended row behind a pad.
    ///
    /// It has an address because the caller is you, not a hook, and
    /// because `anchor` has no `purge` subcommand to defer to
    /// (anchor#14). Moved here from the grid when the right column became
    /// launchers (`docs/18-the-launch-column.md`).
    Purge(Slot),
    /// `tally mute <addr> [--off]` — stop a slot raising attention.
    ///
    /// Rare enough to live in a shell, and necessary enough not to delete:
    /// it is the starvation hatch for `►` and for the tempo
    /// (`docs/16-the-queue.md`, `docs/17-the-tempo.md`).
    Mute(Slot, bool),
}

/// A validated `tally spawn`: a real address and a non-empty command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The pad being claimed. Its page is known to be declared.
    pub at: Slot,
    /// `argv[0]`.
    pub command: String,
    /// `argv[1..]`.
    pub args: Vec<String>,
    /// Spawn and come back, instead of handing this pane over.
    ///
    /// Inside tmux the default is to **adopt**: the pane you ran this in
    /// becomes the view and the shell is replaced
    /// (`docs/15-views.md#adoption-tally-spawn-from-a-pane`). A script
    /// that wants the old spawn-and-return says so. Outside tmux there is
    /// nothing to adopt and this changes nothing.
    pub detach: bool,
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
        "spawn" => claim(args, pages).map(Request::Spawn),
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

fn claim(args: impl Iterator<Item = String>, pages: &Pages) -> Result<Claim, Error> {
    let mut args = args.into_iter();
    let mut detach = false;
    let mut text: Option<String> = None;
    let mut command: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();

    // Everything after `--` is the command. Anything *before* it that is
    // not the address or a flag we have would be silently ignored
    // otherwise, and a spawn that runs something other than what was typed
    // is the one outcome this parser exists to prevent.
    while let Some(token) = args.next() {
        match token.as_str() {
            "--" => {
                if text.is_none() {
                    return Err(Error::Usage(
                        "spawn needs an address before `--`".to_owned(),
                    ));
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
            flag if flag.starts_with('-') => {
                return Err(Error::Usage(format!("unknown flag {flag:?}")));
            }
            _ if text.is_some() => {
                return Err(Error::Usage(format!(
                    "unexpected {token:?}; a command goes after `--`"
                )));
            }
            _ => text = Some(token),
        }
    }

    let Some(text) = text else {
        return Err(Error::Usage("spawn needs an address".to_owned()));
    };
    let at = address(&text, pages)?;
    let command = command.unwrap_or_else(|| DEFAULT_COMMAND.to_owned());
    Ok(Claim {
        at,
        command,
        args: rest,
        detach,
    })
}

/// Parse an address a human typed: `row:col`, or `page:row:col`.
///
/// Two parts means the first page, because most people live on one and
/// typing its name every time is the friction this command exists to
/// remove.
///
/// # Errors
/// [`Error::Address`] for the wrong number of parts, a coordinate outside
/// `1..=8`, or a page no `slots.toml` declares. Never clamped.
pub fn address(text: &str, pages: &Pages) -> Result<Slot, Error> {
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

    fn attention_of(args: &[&str]) -> Result<bool, Error> {
        match parse(args.iter().map(|s| (*s).to_owned()), &named())? {
            Request::Attention(on) => Ok(on),
            other => panic!("asked for attention, got {other:?}"),
        }
    }

    /// Adoption is the default inside tmux, so the flag that turns it off
    /// has to be reachable from either side of the address — a shell user
    /// types it where it occurs to them.
    #[test]
    fn detach_is_accepted_before_or_after_the_address() {
        let pages = named();
        assert!(!spawn(&["spawn", "1:1"], &pages).expect("plain").detach);
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
        let with_cmd =
            spawn(&["spawn", "--detach", "1:1", "--", "ls", "-l"], &pages).expect("with a command");
        assert!(with_cmd.detach);
        assert_eq!(with_cmd.command, "ls");
        assert_eq!(with_cmd.args, ["-l"]);
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
    /// saw, so `tally spawn --detach` alone would have tried to parse the
    /// flag as one.
    #[test]
    fn detach_alone_is_not_an_address() {
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
        assert_eq!(claim.command, DEFAULT_COMMAND);
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
        assert_eq!(crate::slot::parse(&label, &pages), Some(claim.at));
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
        assert_eq!(claim.command, "cargo");
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
        assert!(matches!(
            parse(["ls".to_owned()], &pages),
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
}
