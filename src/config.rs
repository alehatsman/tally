//! `~/.config/tally/slots.toml` — what the pages of the address space are
//! called.
//!
//! One table (`docs/02-model.md`): `[[page]]` is *how an address is
//! spelled*. There used to be two more — `[[spec]]` was *what to run* and
//! `[[slot]]` was *what is where* — and both were cut once the measurement
//! was in: `~/.config/tally/` was empty, nothing had ever reached them, and
//! **cwd is the one fact a config file cannot know in advance and a shell
//! always knows** ([D8](../docs/10-decisions.md#d8--slotstoml-as-the-spawn-path),
//! #40). Spawning is `tally spawn` and the `>` column.
//!
//! **Everything is optional and a missing file is not an error.** No
//! config is eight unnamed pages, which is a fully working grid — it
//! lights up for anything a shell script labelled.
//!
//! A *malformed* file is a different matter and is treated differently:
//! it is reported and then ignored, and tallyd runs on defaults. Refusing
//! to start would take away the watching to punish a mistake in the
//! naming, and watching is the half that is load-bearing.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::slot::{Pages, PAGES};

/// Why a config file was not usable.
#[derive(Debug)]
pub enum Error {
    /// The file could not be read. Absence is **not** this — a missing
    /// file is [`Config::default`] and no error at all.
    Read(std::io::Error),
    /// It is not TOML.
    Syntax(toml::de::Error),
    /// It is TOML and it says something we cannot act on.
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(e) => write!(f, "cannot read: {e}"),
            Self::Syntax(e) => write!(f, "not valid TOML: {e}"),
            Self::Invalid(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

/// A parsed, validated `slots.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pages: Pages,
}

impl Config {
    /// `$XDG_CONFIG_HOME/tally/slots.toml`, else `~/.config/tally/slots.toml`.
    pub fn default_path() -> PathBuf {
        let base = crate::env_path("XDG_CONFIG_HOME").unwrap_or_else(|| {
            crate::env_path("HOME")
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        });
        base.join("tally/slots.toml")
    }

    /// Read and validate, or return defaults if the file is not there.
    ///
    /// # Errors
    /// [`Error::Read`] for an unreadable file that exists, [`Error::Syntax`]
    /// for bad TOML, [`Error::Invalid`] for TOML that says something
    /// impossible.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            // The common case, and not a problem: M1 was config-free and
            // the install still is.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(Error::Read(e)),
        };
        Self::parse(&text)
    }

    /// Validate an in-memory config. Separated from [`Self::load`] so the
    /// rules can be tested without a filesystem.
    ///
    /// # Errors
    /// As [`Self::load`], minus [`Error::Read`].
    pub fn parse(text: &str) -> Result<Self, Error> {
        let file: File = toml::from_str(text).map_err(Error::Syntax)?;
        file.resolve()
    }

    /// The declared page names, in top-row order.
    pub fn pages(&self) -> &Pages {
        &self.pages
    }
}

/// The file as TOML sees it, before any rule has been applied.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    page: Vec<PageEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageEntry {
    name: String,
}

impl File {
    fn resolve(self) -> Result<Config, Error> {
        Ok(Config {
            pages: resolve_pages(&self.page)?,
        })
    }
}

fn resolve_pages(entries: &[PageEntry]) -> Result<Pages, Error> {
    // No `[[page]]` at all keeps M1's behaviour: pages "1".."8", so a
    // `tl.slot=3:2:1` from a shell script still lands somewhere.
    if entries.is_empty() {
        return Ok(Pages::default());
    }
    if entries.len() > PAGES as usize {
        return Err(Error::Invalid(format!(
            "{} pages declared; the address space is {PAGES} pages deep",
            entries.len()
        )));
    }
    let mut seen = BTreeSet::new();
    for entry in entries {
        if entry.name.is_empty() {
            return Err(Error::Invalid("a page has an empty name".to_owned()));
        }
        // A page name is the first field of an address, so a colon in one
        // makes `tl.slot` unparseable -- and it would fail silently, as a
        // pad that never lights.
        if entry.name.contains(':') {
            return Err(Error::Invalid(format!(
                "page {:?}: a name cannot contain ':', which separates an address",
                entry.name
            )));
        }
        // The same name is written into a JSON string in the attention
        // journal and into a tmux window name, and a quote, a backslash
        // or a control character is wrong in at least one of them. The
        // journal serialises properly now (`src/journal.rs`), so this is
        // the second fence rather than the only one: the next string
        // built by hand out of a page name inherits the guarantee instead
        // of having to remember it.
        if let Some(bad) = entry
            .name
            .chars()
            .find(|c| matches!(*c, '"' | '\\') || c.is_control())
        {
            return Err(Error::Invalid(format!(
                "page {:?}: a name cannot contain {bad:?}, which has to survive a JSON string and a tmux window name",
                entry.name
            )));
        }
        if !seen.insert(entry.name.as_str()) {
            return Err(Error::Invalid(format!(
                "page {:?} is declared twice",
                entry.name
            )));
        }
    }
    Ok(Pages::new(entries.iter().map(|p| p.name.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = r#"
[[page]]
name = "work"
[[page]]
name = "infra"
"#;

    #[test]
    fn the_documented_example_parses() {
        let c = Config::parse(FULL).expect("valid");
        assert_eq!(c.pages().index_of("work"), Some(1));
        assert_eq!(c.pages().index_of("infra"), Some(2));
    }

    /// Missing is not malformed. This is the whole M1 install, and still
    /// the whole install.
    #[test]
    fn an_absent_file_is_eight_unnamed_pages() {
        let c = Config::load(Path::new("/nonexistent/tally/slots.toml")).expect("absence is ok");
        assert_eq!(c.pages(), &Pages::default());
        assert_eq!(c.pages().index_of("3"), Some(3));
    }

    #[test]
    fn an_empty_file_is_the_same_as_no_file() {
        assert_eq!(Config::parse("").expect("valid"), Config::default());
    }

    #[test]
    fn a_duplicate_page_name_is_refused() {
        let e = Config::parse("[[page]]\nname = \"p\"\n[[page]]\nname = \"p\"\n")
            .expect_err("duplicate");
        assert!(format!("{e}").contains("twice"), "{e}");
    }

    /// A colon in a page name makes every `tl.slot` on it unparseable,
    /// and it would fail as a pad that simply never lights.
    #[test]
    fn a_page_name_cannot_contain_a_colon() {
        let e = Config::parse("[[page]]\nname = \"a:b\"\n").expect_err("colon");
        assert!(format!("{e}").contains("':'"), "{e}");
    }

    /// A page name is written into the attention journal as part of a
    /// JSON string, and into a tmux window name. A quote in one used to
    /// emit a line `jq` could not read, which costs not that line but
    /// every line after it in the week M1 is measured by.
    #[test]
    fn a_page_name_cannot_contain_a_quote_a_backslash_or_a_control_character() {
        for text in [
            r#"[[page]]
name = "wo\"rk"
"#,
            r#"[[page]]
name = "wo\\rk"
"#,
            r#"[[page]]
name = "wo\nrk"
"#,
        ] {
            let e = Config::parse(text).expect_err("hostile page name");
            assert!(format!("{e}").contains("cannot contain"), "{e}");
        }
    }

    /// Nine pages is a ninth page's worth of addresses that cannot be
    /// written. The limit used to be the eight buttons in the top row;
    /// since `docs/02-model.md` it is the depth of the address space
    /// itself, which is the thing it was always really about.
    #[test]
    fn more_pages_than_the_address_space_is_refused() {
        let text = (1..=9).fold(String::new(), |mut acc, n| {
            use std::fmt::Write as _;
            let _written = writeln!(acc, "[[page]]\nname = \"p{n}\"");
            acc
        });
        let e = Config::parse(&text).expect_err("nine pages");
        assert!(format!("{e}").contains("8 pages deep"), "{e}");
    }

    /// A typo in a key is a silently-ignored line otherwise -- the kind
    /// that has you staring at a dark pad for ten minutes.
    #[test]
    fn an_unknown_key_is_refused_rather_than_ignored() {
        let e = Config::parse("[[page]]\nname = \"p\"\nnmae = \"q\"\n").expect_err("typo");
        assert!(matches!(e, Error::Syntax(_)), "{e}");
    }

    /// A table that was cut is a typo now, and says so rather than being
    /// ignored (#40).
    #[test]
    fn a_cut_table_is_refused_rather_than_ignored() {
        for text in [
            "[[spec]]\nname = \"s\"\nargv = [\"x\"]\n",
            "[[slot]]\nat = \"1:1:1\"\nspec = \"s\"\n",
        ] {
            let e = Config::parse(text).expect_err("cut table");
            assert!(matches!(e, Error::Syntax(_)), "{e}");
        }
    }

    #[test]
    fn broken_toml_is_a_syntax_error_not_a_panic() {
        assert!(matches!(
            Config::parse("[[page").expect_err("broken"),
            Error::Syntax(_)
        ));
    }
}
