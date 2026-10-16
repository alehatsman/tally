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
usage: tally spawn <row:col | page:row:col> [-- command [args...]]

Claim a pad for a session started in the current directory.

  tally spawn 1:1                      claude, here, on page 1
  tally spawn 3:4:4                    claude, here, on page 3
  tally spawn work:3:4 -- cargo test   a different command

The directory is always the one you are standing in, which is the whole
point; there is no --cwd. Two-part addresses are row:col on the first
page. Row 1 is the top, column 1 is the left.
";

/// What the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Print [`USAGE`] and stop.
    Help,
    /// Claim a pad.
    Spawn(Claim),
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
pub fn parse(
    args: impl IntoIterator<Item = String>,
    pages: &Pages,
) -> Result<Request, Error> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err(Error::Usage("no subcommand".to_owned()));
    };
    match first.as_str() {
        "help" | "-h" | "--help" => Ok(Request::Help),
        "spawn" => claim(args, pages).map(Request::Spawn),
        other => Err(Error::Usage(format!("unknown subcommand {other:?}"))),
    }
}

fn claim(mut args: impl Iterator<Item = String>, pages: &Pages) -> Result<Claim, Error> {
    let Some(text) = args.next() else {
        return Err(Error::Usage("spawn needs an address".to_owned()));
    };
    if text == "--" {
        return Err(Error::Usage("spawn needs an address before `--`".to_owned()));
    }
    let at = address(&text, pages)?;

    // Everything after `--` is the command. Anything *before* it that is
    // not the address is a flag we do not have, and silently ignoring it
    // would spawn something other than what was typed.
    let (command, rest) = match args.next() {
        None => (DEFAULT_COMMAND.to_owned(), Vec::new()),
        Some(sep) if sep == "--" => {
            let mut after = args;
            let Some(command) = after.next() else {
                return Err(Error::Usage("`--` with nothing after it".to_owned()));
            };
            (command, after.collect())
        }
        Some(other) => {
            return Err(Error::Usage(format!(
                "unexpected {other:?}; a command goes after `--`"
            )))
        }
    };
    Ok(Claim {
        at,
        command,
        args: rest,
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
            Error::Address(format!("no page named {name:?}; declared: {}", declared(pages)))
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
            Request::Help => panic!("asked to spawn, got help"),
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
        let claim = spawn(&["spawn", "1:1", "--", "cargo", "watch", "-x", "test"], &named())
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
