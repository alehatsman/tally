//! `~/.config/tally/slots.toml` — which pads spawn what.
//!
//! Three tables, each answering a different question
//! (`docs/03-model.md`): `[[page]]` is *where the top row points*,
//! `[[spec]]` is *what to run*, `[[slot]]` is *what is where*.
//!
//! **Everything is optional and a missing file is not an error.** No
//! config is eight unnamed pages and no specs, which is a fully working
//! grid — it lights up for anything a shell script labelled, and only
//! spawning from a dark pad needs the file.
//!
//! A *malformed* file is a different matter and is treated differently:
//! it is reported and then ignored, and tallyd runs on defaults. Refusing
//! to start would take away the watching to punish a mistake in the
//! spawning, and watching is the half that is load-bearing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::slot::{self, Pages, Slot, PAGES};

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

/// Everything a press on a dark pad needs to know.
///
/// Flattened at load time: a `[[slot]]` naming a `[[spec]]` becomes one
/// of these, so nothing downstream has to resolve a name against a table
/// while a finger is waiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The spec's name, which is what goes into `tl.spec` so a respawn
    /// can find it again without the slot table.
    pub spec: String,
    /// `argv[0]`.
    pub command: String,
    /// `argv[1..]`.
    pub args: Vec<String>,
    /// Absolute, with `~` already expanded. `None` means anchord's cwd.
    pub cwd: Option<PathBuf>,
    /// Extra environment for the child.
    pub env: BTreeMap<String, String>,
    /// Run it as a login shell.
    pub login_shell: bool,
}

/// A parsed, validated `slots.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pages: Pages,
    slots: BTreeMap<Slot, Launch>,
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
            // a watching-only install stays that way.
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

    /// What pressing this dark pad spawns, if anything.
    pub fn launch(&self, at: Slot) -> Option<&Launch> {
        self.slots.get(&at)
    }

    /// Find a spec by the name in a session's `tl.spec` label.
    ///
    /// Respawn goes through the **label**, not the slot table, so a
    /// session someone spawned from a shell with `--label tl.spec=claude`
    /// respawns correctly even though no `[[slot]]` mentions its pad.
    pub fn by_spec_name(&self, name: &str) -> Option<&Launch> {
        self.slots.values().find(|l| l.spec == name)
    }

    /// How many slots carry a spec.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether nothing can be spawned from the grid.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// The file as TOML sees it, before any rule has been applied.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    page: Vec<PageEntry>,
    #[serde(default)]
    spec: Vec<SpecEntry>,
    #[serde(default)]
    slot: Vec<SlotEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageEntry {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecEntry {
    name: String,
    argv: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    login_shell: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotEntry {
    at: String,
    spec: String,
    /// Overrides the spec's `cwd`. One spec, many repos is the common
    /// shape — `claude` in eight checkouts is eight slots and one spec.
    #[serde(default)]
    cwd: Option<String>,
}

impl File {
    fn resolve(self) -> Result<Config, Error> {
        let pages = resolve_pages(&self.page)?;
        let specs = index_specs(&self.spec)?;

        let mut slots: BTreeMap<Slot, Launch> = BTreeMap::new();
        for entry in &self.slot {
            let at = slot::parse(&entry.at, &pages).ok_or_else(|| {
                Error::Invalid(format!(
                    "slot {:?}: not page:1-8:1-8, or that page is not declared",
                    entry.at
                ))
            })?;
            let spec = specs.get(entry.spec.as_str()).ok_or_else(|| {
                Error::Invalid(format!(
                    "slot {:?} wants spec {:?}, which no [[spec]] declares",
                    entry.at, entry.spec
                ))
            })?;
            // Two slots on one address is a layout you cannot act on --
            // and unlike two *sessions* on one pad, there is no "newest"
            // to prefer. The file is wrong and says so.
            if slots.contains_key(&at) {
                return Err(Error::Invalid(format!(
                    "slot {:?} is declared twice",
                    entry.at
                )));
            }
            let cwd = entry.cwd.as_ref().or(spec.cwd.as_ref());
            let _previous = slots.insert(
                at,
                Launch {
                    spec: spec.name.clone(),
                    command: command_of(spec)?.to_owned(),
                    args: spec.argv.get(1..).unwrap_or_default().to_vec(),
                    cwd: cwd.map(|c| expand_tilde(c)),
                    env: spec.env.clone(),
                    login_shell: spec.login_shell,
                },
            );
        }
        Ok(Config { pages, slots })
    }
}

fn command_of(spec: &SpecEntry) -> Result<&str, Error> {
    spec.argv
        .first()
        .map(String::as_str)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "spec {:?}: argv is empty, which is not a command",
                spec.name
            ))
        })
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

fn index_specs(entries: &[SpecEntry]) -> Result<BTreeMap<&str, &SpecEntry>, Error> {
    let mut specs: BTreeMap<&str, &SpecEntry> = BTreeMap::new();
    for entry in entries {
        if specs.insert(entry.name.as_str(), entry).is_some() {
            return Err(Error::Invalid(format!(
                "spec {:?} is declared twice",
                entry.name
            )));
        }
    }
    Ok(specs)
}

/// Expand a leading `~`.
///
/// **tally does this, not anchord.** `~` is a shell's idea and nothing
/// between the config file and the PTY is a shell — anchor would take
/// `~/projects` as a relative directory called `~`, and the spawn would
/// fail with a path nobody recognises.
///
/// Only a leading `~/` or a bare `~`. `~alice` is another user's home and
/// resolving it needs the password database; leaving it alone is better
/// than guessing at `/Users/alice`.
fn expand_tilde(path: &str) -> PathBuf {
    let Some(home) = crate::env_path("HOME") else {
        return PathBuf::from(path);
    };
    if path == "~" {
        return home;
    }
    path.strip_prefix("~/")
        .map_or_else(|| PathBuf::from(path), |rest| home.join(rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = r#"
[[page]]
name = "work"
[[page]]
name = "infra"

[[slot]]
at   = "work:3:5"
spec = "claude"
cwd  = "/tmp/anchor"

[[slot]]
at   = "infra:1:1"
spec = "shell"

[[spec]]
name = "claude"
argv = ["claude", "--dangerously-skip-permissions"]
login_shell = true

[[spec]]
name = "shell"
argv = ["zsh"]
cwd  = "/tmp"
"#;

    fn at(page: u8, row: u8, col: u8) -> Slot {
        Slot { page, row, col }
    }

    #[test]
    fn the_documented_example_parses() {
        let c = Config::parse(FULL).expect("valid");
        assert_eq!(c.pages().index_of("work"), Some(1));
        assert_eq!(c.pages().index_of("infra"), Some(2));
        assert_eq!(c.len(), 2);

        let claude = c.launch(at(1, 3, 5)).expect("work:3:5");
        assert_eq!(claude.command, "claude");
        assert_eq!(claude.args, ["--dangerously-skip-permissions"]);
        assert_eq!(claude.cwd.as_deref(), Some(Path::new("/tmp/anchor")));
        assert!(claude.login_shell);
        assert_eq!(claude.spec, "claude");
    }

    /// A slot with no `cwd` of its own inherits the spec's.
    #[test]
    fn a_slot_falls_back_to_its_specs_cwd() {
        let c = Config::parse(FULL).expect("valid");
        let shell = c.launch(at(2, 1, 1)).expect("infra:1:1");
        assert_eq!(shell.cwd.as_deref(), Some(Path::new("/tmp")));
        assert!(!shell.login_shell);
    }

    /// The reason `[[slot]]` and `[[spec]]` are two tables: one spec,
    /// many repos is the shape this is for.
    #[test]
    fn one_spec_serves_many_slots() {
        let c = Config::parse(
            r#"
[[page]]
name = "p"
[[spec]]
name = "claude"
argv = ["claude"]
[[slot]]
at = "p:1:1"
spec = "claude"
cwd = "/tmp/a"
[[slot]]
at = "p:1:2"
spec = "claude"
cwd = "/tmp/b"
"#,
        )
        .expect("valid");
        assert_eq!(
            c.launch(at(1, 1, 1)).expect("a").cwd.as_deref(),
            Some(Path::new("/tmp/a"))
        );
        assert_eq!(
            c.launch(at(1, 1, 2)).expect("b").cwd.as_deref(),
            Some(Path::new("/tmp/b"))
        );
    }

    /// Missing is not malformed. This is the whole M1 install.
    #[test]
    fn an_absent_file_is_eight_unnamed_pages_and_nothing_to_spawn() {
        let c = Config::load(Path::new("/nonexistent/tally/slots.toml")).expect("absence is ok");
        assert_eq!(c.pages(), &Pages::default());
        assert!(c.is_empty());
        assert_eq!(c.pages().index_of("3"), Some(3));
    }

    #[test]
    fn an_empty_file_is_the_same_as_no_file() {
        assert_eq!(Config::parse("").expect("valid"), Config::default());
    }

    /// A pad that never lights because its page was never declared is the
    /// worst failure this file has, so it is the one that must not parse.
    #[test]
    fn a_slot_on_an_undeclared_page_is_refused() {
        let e = Config::parse(
            r#"
[[spec]]
name = "s"
argv = ["x"]
[[slot]]
at = "nowhere:1:1"
spec = "s"
"#,
        )
        .expect_err("undeclared page");
        assert!(matches!(e, Error::Invalid(_)), "{e}");
    }

    #[test]
    fn a_slot_naming_an_undeclared_spec_is_refused() {
        let e = Config::parse(
            r#"
[[page]]
name = "p"
[[slot]]
at = "p:1:1"
spec = "ghost"
"#,
        )
        .expect_err("undeclared spec");
        assert!(format!("{e}").contains("ghost"), "{e}");
    }

    #[test]
    fn two_slots_on_one_address_is_refused() {
        let e = Config::parse(
            r#"
[[page]]
name = "p"
[[spec]]
name = "s"
argv = ["x"]
[[slot]]
at = "p:2:2"
spec = "s"
[[slot]]
at = "p:2:2"
spec = "s"
"#,
        )
        .expect_err("duplicate");
        assert!(format!("{e}").contains("twice"), "{e}");
    }

    #[test]
    fn a_duplicate_page_or_spec_name_is_refused() {
        for text in [
            "[[page]]\nname = \"p\"\n[[page]]\nname = \"p\"\n",
            "[[spec]]\nname = \"s\"\nargv = [\"x\"]\n[[spec]]\nname = \"s\"\nargv = [\"y\"]\n",
        ] {
            let e = Config::parse(text).expect_err("duplicate");
            assert!(format!("{e}").contains("twice"), "{e}");
        }
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
    /// since `docs/16` it is the depth of the address space itself, which
    /// is the thing it was always really about.
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

    /// An empty argv would otherwise reach anchord as `command: ""`.
    #[test]
    fn an_empty_argv_is_not_a_command() {
        let e = Config::parse(
            r#"
[[page]]
name = "p"
[[spec]]
name = "s"
argv = []
[[slot]]
at = "p:1:1"
spec = "s"
"#,
        )
        .expect_err("empty argv");
        assert!(format!("{e}").contains("argv"), "{e}");
    }

    /// A typo in a key is a silently-ignored line otherwise -- the kind
    /// that has you staring at a dark pad for ten minutes.
    #[test]
    fn an_unknown_key_is_refused_rather_than_ignored() {
        let e = Config::parse("[[spec]]\nname = \"s\"\nargv = [\"x\"]\ncdw = \"/tmp\"\n")
            .expect_err("typo");
        assert!(matches!(e, Error::Syntax(_)), "{e}");
    }

    #[test]
    fn broken_toml_is_a_syntax_error_not_a_panic() {
        assert!(matches!(
            Config::parse("[[page").expect_err("broken"),
            Error::Syntax(_)
        ));
    }

    #[test]
    fn a_leading_tilde_is_expanded_and_nothing_else_is() {
        let home = crate::env_path("HOME").expect("HOME");
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/p/anchor"), home.join("p/anchor"));
        // Another user's home needs the password database; guessing at
        // `/Users/alice` is worse than passing it through.
        assert_eq!(expand_tilde("~alice/p"), PathBuf::from("~alice/p"));
        assert_eq!(expand_tilde("/abs/p"), PathBuf::from("/abs/p"));
        assert_eq!(expand_tilde("rel/p"), PathBuf::from("rel/p"));
        // Not a path component in the middle.
        assert_eq!(expand_tilde("/a/~/b"), PathBuf::from("/a/~/b"));
    }

    /// Respawn reads `tl.spec` off the session, so the lookup has to work
    /// by name as well as by address.
    #[test]
    fn a_spec_is_findable_by_the_name_a_label_carries() {
        let c = Config::parse(FULL).expect("valid");
        assert_eq!(
            c.by_spec_name("claude").map(|l| l.command.as_str()),
            Some("claude")
        );
        assert_eq!(c.by_spec_name("nope"), None);
    }

    /// A spec nothing points at is dead weight, not an error: you are
    /// mid-edit, and refusing to load would take the whole grid with it.
    #[test]
    fn an_unused_spec_is_allowed() {
        let c = Config::parse("[[spec]]\nname = \"idle\"\nargv = [\"x\"]\n").expect("valid");
        assert!(c.is_empty());
    }
}
