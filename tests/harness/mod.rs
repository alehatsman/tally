//! A throwaway tmux server, a fake `anchor`, and the two reads every
//! scenario makes.
//!
//! Shared by the integration tests, which is why it is a module rather
//! than a file of its own: `tests/*.rs` are separate crates, and `mod
//! harness;` is how one of them borrows this.

// A test's failure report *is* a panic, and `clippy::panic` is a
// restriction lint aimed at the daemon rather than at the suite that
// drives it.
#![allow(clippy::panic, reason = "a failed assertion is how a test reports")]
#![allow(dead_code, reason = "each test crate uses a subset of this")]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tally::jump::Tmux;

/// Serialises the whole suite.
///
/// `TALLY_TMUX_SOCKET` is how a `Tmux` is pointed at a throwaway server
/// and it is read from the **process** environment, so two tests building
/// one at the same time would race over which socket both of them get.
/// Held for the lifetime of a [`Server`], which is the lifetime of a test.
static GATE: Mutex<()> = Mutex::new(());

/// Makes each socket name unique within the run.
static NEXT: Mutex<u32> = Mutex::new(0);

/// Is tmux here at all?
///
/// The suite is the first thing in this repo to need it, so a machine
/// without it must still get a green `cargo test` — see [`skip`] for the
/// price of that and how it is paid.
#[must_use]
pub(crate) fn tmux_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Say, loudly, that a test did not run.
///
/// **A skip that whispers is the pattern this repo's roadmap complains
/// about** — a green run that did not run. `cargo test` hides stdout for
/// passing tests, so this goes to stderr and is shaped to be seen in a
/// scroll of dots.
pub(crate) fn skip(what: &str) {
    eprintln!("\n!! SKIPPED {what}: tmux is not on PATH.");
    eprintln!("!! This suite is the only thing in this repo that can see what tmux does.");
    eprintln!("!! A green run without it is green about strictly less (#21).\n");
}

/// A server config worth running every scenario against.
///
/// Two variables, both measured as mattering:
///
/// - **`renumber-windows`** is [#20](https://github.com/alehatsman/tally/issues/20):
///   the owner's `~/.tmux.conf` sets it, a named socket is a separate
///   *server* and not a separate *config*, and with it on, closing a
///   window shifts every sibling to its right — so a view is at the wrong
///   index with no Move involved. `Tmux::hold_indexes` exists to pin it
///   off, and the `On` row is the only thing that can tell whether it
///   works.
/// - **`base-index`** because `docs/31` records the placeholder session
///   landing at 0 *and* at 1, and which one it is decides whether the `-k`
///   has already replaced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Config {
    /// What to call it when a scenario fails under one config and not
    /// another, which is the whole reason for the matrix.
    pub(crate) name: &'static str,
    /// `renumber-windows on` in the file the server reads.
    pub(crate) renumber: bool,
    /// What the server's `base-index` is set to.
    pub(crate) base_index: u8,
}

/// Every config, and every scenario runs under all of them.
pub(crate) const CONFIGS: [Config; 4] = [
    Config {
        name: "tmux defaults",
        renumber: false,
        base_index: 0,
    },
    Config {
        name: "base-index 1",
        renumber: false,
        base_index: 1,
    },
    Config {
        name: "renumber-windows on",
        renumber: true,
        base_index: 0,
    },
    Config {
        name: "renumber-windows on, base-index 1",
        renumber: true,
        base_index: 1,
    },
];

/// Run `body` once per [`CONFIGS`] entry, each against a fresh server.
///
/// Skips loudly and passes if tmux is absent, which is the one concession
/// to a machine that does not have it.
pub(crate) fn each_config(what: &str, body: impl Fn(&Server)) {
    if !tmux_available() {
        skip(what);
        return;
    }
    for config in CONFIGS {
        let server = Server::start(config);
        body(&server);
    }
}

/// A tmux server that exists for one test and is killed with it.
#[derive(Debug)]
pub(crate) struct Server {
    /// The `-L` name. Unique per server, so a leaked one cannot be
    /// mistaken for the next test's.
    socket: String,
    /// The `anchor` a view runs. A real one is not wanted and not here.
    anchor: PathBuf,
    /// Deleted on drop, which takes the config file and the fake anchor
    /// with it.
    dir: PathBuf,
    /// Which [`Config`] this was started with, for failure messages.
    pub(crate) config: Config,
    /// Held so that no other test is setting `TALLY_TMUX_SOCKET` while
    /// this one is using it.
    _gate: MutexGuard<'static, ()>,
}

impl Server {
    /// Start a server with `config` **loaded from a file we wrote**.
    ///
    /// `-f` is the point, not a detail: tmux reads `~/.tmux.conf` when it
    /// starts a server, so without it this suite would be testing the
    /// machine it runs on. A config is read once, at server start, and
    /// every later `tmux -L <socket>` joins the server that already has
    /// it — which is also why the server is started here and never left
    /// to be auto-started by the code under test.
    #[must_use]
    pub(crate) fn start(config: Config) -> Self {
        let gate = GATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let n = {
            let mut next = NEXT
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *next += 1;
            *next
        };
        let socket = format!("tally-test-{}-{n}", std::process::id());
        let dir = std::env::temp_dir().join(&socket);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let conf = dir.join("tmux.conf");
        let mut f = std::fs::File::create(&conf).expect("write config");
        writeln!(f, "set -g base-index {}", config.base_index).expect("write config");
        writeln!(f, "set -g pane-base-index {}", config.base_index).expect("write config");
        if config.renumber {
            writeln!(f, "set -g renumber-windows on").expect("write config");
        }
        drop(f);

        let anchor = write_fake_anchor(&dir);

        // A session, not a bare `start-server`: a server with nothing in
        // it is entitled to exit, and this one has to outlive the command
        // that made it. It is also a pane nothing tagged, which is what
        // every pane on a real server that tally did not make looks like.
        let started = Command::new("tmux")
            .args(["-L", &socket, "-f"])
            .arg(&conf)
            .args(["new-session", "-d", "-s", "harness", "sleep 100000"])
            .output()
            .expect("start tmux");
        assert!(
            started.status.success(),
            "could not start tmux on {socket}: {}",
            String::from_utf8_lossy(&started.stderr)
        );

        // Both are read when a `Tmux` is built or a jump is run, and both
        // are process-global, which is what `GATE` is protecting.
        std::env::set_var("TALLY_TMUX_SOCKET", &socket);
        // Without this a headless `jump` would run `open -a Alacritty` on
        // the machine running the suite. `TALLY_TERMINAL` is left unset so
        // that `raise()` has no app to raise and does nothing at all.
        std::env::set_var("TALLY_TERMINAL_CMD", "true");
        std::env::remove_var("TALLY_TERMINAL");

        Self {
            socket,
            anchor,
            dir,
            config,
            _gate: gate,
        }
    }

    /// A `Tmux` pointed at this server, running the fake anchor.
    #[must_use]
    pub(crate) fn tmux(&self) -> Tmux {
        Tmux::with_anchor(self.anchor.to_string_lossy().into_owned())
    }

    /// Run a tmux command against this server and return stdout.
    ///
    /// For the test's own reads and for arranging a world the code under
    /// test did not make. Panics on failure, which in a test is the
    /// report.
    pub(crate) fn sh(&self, args: &[&str]) -> String {
        let out = Command::new("tmux")
            .args(["-L", &self.socket])
            .args(args)
            .output()
            .expect("run tmux");
        assert!(
            out.status.success(),
            "tmux {args:?} failed under {}: {}",
            self.config.name,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// Run a tmux command and report whether it worked, without panicking.
    ///
    /// `Err` holds stderr — which is where `index N in use` arrives, and
    /// that string is the symptom half this suite exists for.
    ///
    /// # Errors
    /// What tmux said.
    pub(crate) fn try_sh(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("tmux")
            .args(["-L", &self.socket])
            .args(args)
            .output()
            .expect("run tmux");
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
        }
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }

    /// Every pane on the server, with the two things tally writes and the
    /// place it actually sits.
    ///
    /// **The claim and the place are different questions**, and a drifted
    /// view is exactly where they disagree (`docs/31`), so both are read
    /// here and never inferred from each other.
    #[must_use]
    pub(crate) fn panes(&self) -> Vec<PaneRow> {
        self.sh(&[
            "list-panes",
            "-a",
            "-F",
            "#{pane_id}|#{session_name}|#{window_index}|#{pane_dead}|#{@tally_session}|#{@tally_slot}",
        ])
        .lines()
        .filter_map(|l| {
            let mut f = l.split('|');
            Some(PaneRow {
                id: f.next()?.to_owned(),
                tmux_session: f.next()?.to_owned(),
                index: f.next()?.to_owned(),
                dead: f.next()? == "1",
                session: f.next()?.to_owned(),
                slot: f.next()?.to_owned(),
            })
        })
        .collect()
    }

    /// The pane viewing `session`, by the tag tally writes.
    ///
    /// # Panics
    /// When there is no such view, which in these tests means the thing
    /// under test lost one.
    #[must_use]
    pub(crate) fn view(&self, session: &str) -> PaneRow {
        self.panes()
            .into_iter()
            .find(|p| p.session == session)
            .unwrap_or_else(|| {
                panic!(
                    "no view for {session} under {}; panes: {:#?}",
                    self.config.name,
                    self.panes()
                )
            })
    }

    /// Is there a view for `session` at all?
    #[must_use]
    pub(crate) fn has_view(&self, session: &str) -> bool {
        self.panes().iter().any(|p| p.session == session)
    }

    /// Does a tmux session by this name exist here?
    #[must_use]
    pub(crate) fn session_exists(&self, name: &str) -> bool {
        Command::new("tmux")
            .args(["-L", &self.socket, "has-session", "-t", name])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    /// Wait until `pane` is a corpse, or give up.
    ///
    /// A pane whose command has exited becomes dead on tmux's own clock,
    /// not ours, and `remain-on-exit` is what keeps it there to be seen.
    ///
    /// # Panics
    /// If it never dies, because every later assertion would then be
    /// about the wrong world.
    pub(crate) fn wait_until_dead(&self, pane: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.panes().iter().any(|p| p.id == pane && p.dead) {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("{pane} never died under {}", self.config.name);
    }

    /// Where a scenario's failure message should start: everything, dumped.
    #[must_use]
    pub(crate) fn dump(&self) -> String {
        format!("under {}: {:#?}", self.config.name, self.panes())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _killed = Command::new("tmux")
            .args(["-L", &self.socket, "kill-server"])
            .output();
        let _gone = std::fs::remove_dir_all(&self.dir);
    }
}

/// One line of `list-panes`, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaneRow {
    /// `%12`.
    pub(crate) id: String,
    /// The row session it is in — where it **is**.
    pub(crate) tmux_session: String,
    /// The window index it is at — the column it is actually in.
    pub(crate) index: String,
    /// Its command has exited and `remain-on-exit` kept the pane.
    pub(crate) dead: bool,
    /// `@tally_session`: the anchor session this views.
    pub(crate) session: String,
    /// `@tally_slot`: the address it **claims**.
    pub(crate) slot: String,
}

impl PaneRow {
    /// Is this pane where `session`:`index` says it should be?
    #[must_use]
    pub(crate) fn at(&self, session: &str, index: u8) -> bool {
        self.tmux_session == session && self.index == index.to_string()
    }
}

/// Write the three-line `anchor` these tests run instead of the real one.
///
/// `attach <id>` is the only verb a view uses. It sleeps, so the pane
/// stays alive and can be moved around like the thing it stands in for —
/// except for an id starting `dead`, which exits immediately and is how a
/// corpse is made on purpose.
fn write_fake_anchor(dir: &Path) -> PathBuf {
    let path = dir.join("anchor");
    let script = "#!/bin/sh\n\
         # attach <id>: a view's whole command.\n\
         case \"$2\" in\n\
         dead*) exit 0 ;;\n\
         *) exec sleep 100000 ;;\n\
         esac\n";
    std::fs::write(&path, script).expect("write fake anchor");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake anchor");
    }
    path
}
