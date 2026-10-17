//! A client for anchord's control socket.
//!
//! anchor owns the PTYs; tally owns a grid of LEDs and an opinion about
//! what they mean (`docs/07-seams.md`). This speaks the newline-JSON half
//! of the protocol — enough to reconstruct the grid every tick.
//!
//! Control calls are **one connection each**. A unix socket is cheap, and
//! it makes "anchord is down" a clean per-tick error rather than a
//! connection to nurse back to health: the grid freezes and the logo goes
//! red, which is the designed behaviour.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::signal::SessionState;

/// Label prefix. Ours, and **only** ours.
///
/// teleport writes `teleport.*`; neither side ever writes the other's
/// prefix, including "helpfully" copying one across. A constant rather
/// than a literal at each call site, so that rule is greppable.
pub const PREFIX: &str = "tl.";
/// Which pad a session is bound to: `page:row:col`.
pub const LABEL_SLOT: &str = "tl.slot";
/// Which spec spawned it, for respawn.
pub const LABEL_SPEC: &str = "tl.spec";
/// Set to suppress attention for this session.
pub const LABEL_MUTE: &str = "tl.mute";
/// Set by a `Notification` hook to declare the agent is waiting.
pub const LABEL_ATTENTION: &str = "tl.attention";
/// The Claude Code session id, as that agent's own hook reported it.
///
/// The argument to `claude --resume`, and the only id that resumes
/// anything: anchor's id names a PTY that is gone by the time you want
/// it (`docs/26-the-dead-pane.md`).
///
/// **Never removed.** A conversation outlives the PTY that ran it, which
/// is the whole reason to want it, so the label stays on the dead row.
pub const LABEL_CLAUDE: &str = "tl.claude";
/// Set by tally immediately before it terminates a session.
///
/// anchord records a terminate as an ordinary non-zero exit — there is
/// no field saying who asked, and there should not be: that is the
/// client's opinion, not the daemon's fact. Without this label a kill you
/// performed deliberately renders identically to an agent that crashed,
/// and red stops meaning *go and look*.
pub const LABEL_KILLED: &str = "tl.killed";

/// How long to wait on anchord before calling the tick failed.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Bytes in a frame header: u8 tag, u64 offset BE, u32 length BE.
const HEADER_LEN: usize = 13;

/// The largest frame anchord can legitimately send an observer.
///
/// Live output is one PTY chunk, 64 KiB. The frames right after an attach
/// are catch-up, and those are bigger: anchord replays history one round
/// at a time, and a round is one eighth of a subscriber's 8 MiB queue
/// budget — 1 MiB (anchor `docs/03-pty.md`, `docs/04-protocol.md`).
/// anchord's own client derives its cap from that same constant.
///
/// A cap at all, against a trusted local peer, because the `u32` on the
/// wire is 4 GiB wide and we allocate on it before reading a byte of
/// payload. anchord caps *inbound* frames at one chunk and drops the
/// connection on the header alone, and argues it in the protocol doc: a
/// length is an unverified claim even when the sender is not hostile. The
/// claim we would be trusting is a desynchronised stream — terminal
/// output read as if it were a header — which is a 4 GiB allocation and
/// an abort instead of a watcher that quietly restarts.
const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Why a call did not produce sessions.
#[derive(Debug)]
pub enum Error {
    /// The socket was not there, or would not talk. Usually: anchord is
    /// down. Distinct from an empty session list, and the surface must
    /// show the difference — sixty-four dark pads mean "nothing is
    /// running", never "I cannot tell".
    Unreachable(String),
    /// anchord answered, but not with something we understand.
    Protocol(String),
    /// anchord understood and said no.
    ///
    /// Kept apart from [`Self::Protocol`] because the codes are a
    /// contract and some of them are ordinary: `already_exited` from a
    /// `purge` means the session is still running, which is a thing a
    /// finger does, not a thing that is broken. Only `internal` is
    /// anchord's fault and worth retrying
    /// (anchor `docs/04-protocol.md`).
    Refused {
        /// anchor's error code — `not_found`, `already_exited`, …
        code: String,
        /// Its message, verbatim.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreachable(e) => write!(f, "anchord unreachable: {e}"),
            Self::Protocol(e) => write!(f, "anchord protocol: {e}"),
            Self::Refused { code, message } => write!(f, "anchord refused: {code}: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// One session, as anchord reports it. Fields tally does not use are not
/// deserialised — anchor is free to add more without breaking us.
#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    /// ULID.
    pub id: String,
    /// `running` | `closing` | `exited` | `lost`.
    pub state: String,
    /// Present once a terminal state anchor can account for is reached.
    #[serde(default)]
    pub exit_code: Option<i32>,
    /// Why it is `lost`, when it is.
    #[serde(default)]
    pub lost_reason: Option<String>,
    /// Caller-owned key/value. anchor never interprets these.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    /// Authoritative stream length; never rewinds.
    #[serde(default)]
    pub output_bytes: u64,
    /// The agent's own pid, as anchord forked it. `None` once it is gone.
    ///
    /// Read by nothing on the grid — it exists so a hook can find the
    /// session it is running inside by walking its own process ancestry,
    /// which is the only key that survives two agents in one repo
    /// (`src/attention.rs`, `docs/14-attention.md`).
    #[serde(default)]
    pub pid: Option<i32>,
    /// The directory it was started in. **Kept on dead rows too**, which
    /// is what lets `>` spawn into a row without a config file
    /// (`docs/18-the-launch-column.md`).
    #[serde(default)]
    pub cwd: Option<String>,
    /// `argv[0]`. Inherited by `>`; the arguments deliberately are not.
    #[serde(default)]
    pub command: Option<String>,
    /// Whether anchord wrapped it in a login shell.
    #[serde(default)]
    pub login_shell: bool,
}

impl Session {
    /// This session's lifecycle state, as the renderer wants it.
    ///
    /// An unrecognised string is treated as `Lost` rather than guessed at:
    /// a state we do not know is precisely a state we cannot account for.
    pub fn lifecycle(&self) -> SessionState {
        match self.state.as_str() {
            "running" => SessionState::Running,
            "closing" => SessionState::Closing,
            "exited" => SessionState::Exited,
            _ => SessionState::Lost,
        }
    }

    /// The raw `tl.slot` value, if this session claims a pad.
    pub fn slot_label(&self) -> Option<&str> {
        self.labels.get(LABEL_SLOT).map(String::as_str)
    }

    /// Whether attention is suppressed for this session.
    pub fn muted(&self) -> bool {
        self.labels.contains_key(LABEL_MUTE)
    }

    /// Whether a hook has declared this session is waiting on a human.
    pub fn declared_attention(&self) -> bool {
        self.labels.contains_key(LABEL_ATTENTION)
    }

    /// Did this row become `lost` because anchord itself restarted?
    ///
    /// The distinction matters because `lost` normally means *the process
    /// may still be out there* — a kill that was never observed
    /// completing. A daemon restart is the one reason that cannot mean
    /// that: anchord owns the PTYs, so anchord going away takes them with
    /// it, and the daemon is the one saying so
    /// ([#9](https://github.com/alehatsman/tally/issues/9)).
    pub fn lost_to_restart(&self) -> bool {
        self.lost_reason.as_deref() == Some("daemon_restart")
    }

    /// The conversation this session is running, if its hook has said.
    ///
    /// `None` for anything that is not Claude Code, and for a Claude that
    /// has not reached its first turn — the hook that reports it is the
    /// attention one (`docs/26-the-dead-pane.md`).
    pub fn claude(&self) -> Option<&str> {
        self.labels.get(LABEL_CLAUDE).map(String::as_str)
    }

    /// Whether tally ended this session on purpose.
    pub fn killed(&self) -> bool {
        self.labels.contains_key(LABEL_KILLED)
    }
}

#[derive(Debug, Deserialize)]
struct AttachReply {
    ok: bool,
    #[serde(default)]
    offset: u64,
}

/// What anchord sends an attached observer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// PTY bytes, with the offset of the first one.
    Output {
        /// Offset of the first byte in the stream.
        offset: u64,
        /// The bytes themselves. Not UTF-8 in general.
        bytes: Vec<u8>,
    },
    /// The session ended. `None` when anchor cannot say with what.
    Exit(Option<i32>),
    /// A frame we do not act on — lease, resize.
    Other,
}

/// A stream of frames from an attached session.
#[derive(Debug)]
pub struct Frames {
    reader: BufReader<UnixStream>,
    offset: u64,
}

impl Frames {
    /// Where anchord said the stream starts.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// The next frame, or `None` when the connection ends.
    ///
    /// Ending is **normal**: an exit frame, EOF, or being dropped for
    /// falling behind all finish a watcher, and the supervisor starts a
    /// fresh one if the session is still live.
    pub fn next_frame(&mut self) -> Option<Frame> {
        let mut header = [0u8; HEADER_LEN];
        self.reader.read_exact(&mut header).ok()?;
        let (tag, offset, len) = decode_header(&header)?;
        let mut payload = vec![0u8; len];
        self.reader.read_exact(&mut payload).ok()?;
        Some(match tag {
            0x01 => Frame::Output {
                offset,
                bytes: payload,
            },
            0x03 => Frame::Exit(
                payload
                    .get(0..4)
                    .and_then(|b| b.try_into().ok())
                    .map(i32::from_le_bytes),
            ),
            _ => Frame::Other,
        })
    }
}

/// Splits a frame header into tag, offset and payload length.
///
/// u8 tag, u64 offset BE, u32 length BE. The exit payload inside is
/// little-endian -- mixed, deliberately, and only the code says so.
///
/// `None` means stop reading this connection, which is what the caller
/// does with it: a length past [`MAX_FRAME_BYTES`] is a stream we are no
/// longer in step with, and there is nothing to resynchronise on — the
/// framing is the only boundary a stream has. Ending the watcher is the
/// cheap recovery, because the supervisor already restarts one that ends
/// (`src/watch.rs`), and it re-attaches from its last offset.
///
/// A free function so the decision is testable without a live daemon.
fn decode_header(header: &[u8; HEADER_LEN]) -> Option<(u8, u64, usize)> {
    let tag = *header.first()?;
    let offset = u64::from_be_bytes(header.get(1..9)?.try_into().ok()?);
    let len = usize::try_from(u32::from_be_bytes(header.get(9..13)?.try_into().ok()?)).ok()?;
    (len <= MAX_FRAME_BYTES).then_some((tag, offset, len))
}

#[derive(Debug, Deserialize)]
struct ListReply {
    ok: bool,
    #[serde(default)]
    sessions: Vec<Session>,
    #[serde(default)]
    error: Option<ReplyError>,
}

/// What to run, and what to label it with.
///
/// Borrowed rather than owned: it is built from a config entry and a slot
/// address, used once, and thrown away. Nothing here outlives the call.
#[derive(Debug)]
pub struct Spawn<'a> {
    /// `argv[0]`.
    pub command: &'a str,
    /// `argv[1..]`.
    pub args: &'a [String],
    /// Absolute. **`~` is expanded before it gets here** — nothing between
    /// the config file and the PTY is a shell.
    pub cwd: Option<&'a Path>,
    /// Extra environment for the child.
    pub env: &'a BTreeMap<String, String>,
    /// Run the command as a login shell.
    pub login_shell: bool,
    /// `tl.*` labels, applied at create time.
    pub labels: &'a BTreeMap<String, String>,
}

/// The common shape of every control reply we send.
#[derive(Debug, Deserialize)]
struct OpReply {
    ok: bool,
    #[serde(default)]
    session: Option<String>,
    #[serde(default)]
    error: Option<ReplyError>,
}

#[derive(Debug, Deserialize)]
struct ReplyError {
    code: String,
    message: String,
}

/// A handle to anchord's socket.
#[derive(Debug, Clone)]
pub struct Client {
    path: PathBuf,
}

impl Client {
    /// Point at a socket.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// `$XDG_RUNTIME_DIR/anchor.sock`, else
    /// `$XDG_DATA_HOME/anchor/anchor.sock`, else `~/.local/share/...`.
    pub fn default_path() -> PathBuf {
        if let Some(run) = crate::env_path("XDG_RUNTIME_DIR") {
            return run.join("anchor.sock");
        }
        let data = crate::env_path("XDG_DATA_HOME").unwrap_or_else(|| {
            crate::env_path("HOME")
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local/share")
        });
        data.join("anchor/anchor.sock")
    }

    /// Where this client is pointed.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every session anchord knows about.
    ///
    /// **Unfiltered, by necessity.** anchor's filter is exact match on a
    /// value, AND across keys, and `tl.slot`'s value differs for every pad
    /// — so there is no server-side query for "has a `tl.slot`". Adding a
    /// constant owner label purely to make one work would make that label
    /// mandatory, and `anchor create --label tl.slot=… -- claude` typed at
    /// a shell would silently not light a pad. One label is the binding;
    /// the predicate is ours.
    ///
    /// # Errors
    /// [`Error::Unreachable`] if anchord cannot be reached — which is a
    /// different fact from an empty list, and renders differently.
    pub fn list(&self) -> Result<Vec<Session>, Error> {
        let reply = self.call(r#"{"id":1,"op":"list","filter":{}}"#)?;
        let parsed: ListReply =
            serde_json::from_str(&reply).map_err(|e| Error::Protocol(e.to_string()))?;
        if parsed.ok {
            return Ok(parsed.sessions);
        }
        Err(Error::Protocol(parsed.error.map_or_else(
            || "not ok, and no error given".to_owned(),
            |e| format!("{}: {}", e.code, e.message),
        )))
    }

    /// Attach as an **observer** and return the stream of output frames.
    ///
    /// Never `control: true`. The lease belongs to the terminal you jumped
    /// to; a surface that steals it makes typing stop working, from across
    /// the room, for no visible reason (`docs/04-signals.md`).
    ///
    /// `from` is a byte offset. **Zero the first time**, always: the
    /// signals worth having are emitted at startup — an agent sets its
    /// title and rings in its opening banner — and a supervisor cannot
    /// attach before that happened. Starting at the head reliably misses
    /// exactly the output that matters.
    ///
    /// # Errors
    /// [`Error::Unreachable`] if anchord will not talk, [`Error::Protocol`]
    /// if it refuses the attach.
    pub fn attach(&self, session: &str, from: u64) -> Result<Frames, Error> {
        let stream =
            UnixStream::connect(&self.path).map_err(|e| Error::Unreachable(e.to_string()))?;
        // The handshake is request/response like any other control call, so
        // it gets the same bound. Unbounded, a daemon that accepts the
        // connection and then says nothing parks this thread forever -- and
        // a parked thread is indistinguishable from a healthy watcher to the
        // supervisor, which marks the session watched before the thread
        // starts and only unmarks it when the thread ends (`src/watch.rs`).
        // That pad loses bell detection for the life of the process, with no
        // log line to say so.
        stream
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        stream
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        let request = serde_json::json!({
            "id": 1, "op": "attach", "session": session, "from": from, "control": false
        });
        let mut writing = &stream;
        writing
            .write_all(format!("{request}\n").as_bytes())
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        writing
            .flush()
            .map_err(|e| Error::Unreachable(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        let reply: AttachReply =
            serde_json::from_str(&line).map_err(|e| Error::Protocol(e.to_string()))?;
        if !reply.ok {
            return Err(Error::Protocol(format!("attach refused: {line}")));
        }
        // Past the handshake the bound stops being correct and becomes a
        // bug: the connection streams frames now, and silence is a healthy
        // session whose agent is waiting on a human -- which is precisely
        // the state the surface exists to show. Blocking indefinitely
        // between frames is the intended behaviour
        // ([`Frames::next_frame`]); a read timeout left in place would end
        // every quiet watcher every two seconds. `None` is the blocking
        // default, restored.
        reader
            .get_ref()
            .set_read_timeout(None)
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        Ok(Frames {
            reader,
            offset: reply.offset,
        })
    }

    /// Spawn a session and claim a slot with it.
    ///
    /// The labels go on at **create time**, not in a `label` call
    /// afterwards: a session that exists for even one tick without its
    /// `tl.slot` is a session the next reconcile does not draw, and a pad
    /// that blinks on a second after you pressed it looks like a bug
    /// whether or not it is one.
    ///
    /// `cols`/`rows` are left to anchor's defaults on purpose — the view
    /// window owns the size and `anchor attach` sets it
    /// (`docs/03-model.md`).
    ///
    /// # Errors
    /// [`Error::Refused`] with `invalid_spec` or `spawn_failed` if the
    /// command is not runnable; [`Error::Unreachable`] if anchord is down.
    pub fn create(&self, spawn: &Spawn<'_>) -> Result<String, Error> {
        let request = serde_json::json!({
            "id": 1,
            "op": "create",
            "spec": {
                "command": spawn.command,
                "args": spawn.args,
                "cwd": spawn.cwd,
                "env": spawn.env,
                "login_shell": spawn.login_shell,
            },
            "labels": spawn.labels,
        });
        let reply = self.op(&request)?;
        reply
            .session
            .ok_or_else(|| Error::Protocol("create said ok and named no session".to_owned()))
    }

    /// End a session.
    ///
    /// anchor makes this synchronous to terminal state: when it answers,
    /// the child is dead *and* the exit is recorded
    /// (anchor `docs/04-protocol.md`). That is what lets the pad go red at
    /// the moment it is actually dead rather than when we asked.
    ///
    /// # Errors
    /// [`Error::Refused`] with `not_found`; [`Error::Unreachable`].
    pub fn terminate(&self, session: &str) -> Result<(), Error> {
        let request = serde_json::json!({"id": 1, "op": "terminate", "session": session});
        self.op(&request).map(|_reply| ())
    }

    /// Forget a session — its row, its labels and its output.
    ///
    /// # Errors
    /// [`Error::Refused`] with `already_exited` if it is still running
    /// (anchor's name for "it has not exited", which reads backwards and
    /// is theirs to name), or `not_found` for an id that never existed.
    pub fn purge(&self, session: &str) -> Result<(), Error> {
        let request = serde_json::json!({"id": 1, "op": "purge", "session": session});
        self.op(&request).map(|_reply| ())
    }

    /// Set and remove labels in one call.
    ///
    /// One call rather than two because anchor applies a `label` op as a
    /// unit, and a mute that is half-applied is a mute you cannot reason
    /// about.
    ///
    /// # Errors
    /// [`Error::Refused`] with `not_found` or `invalid_label`.
    pub fn label(
        &self,
        session: &str,
        set: &BTreeMap<String, String>,
        remove: &[&str],
    ) -> Result<(), Error> {
        let request = serde_json::json!({
            "id": 1, "op": "label", "session": session, "set": set, "remove": remove,
        });
        self.op(&request).map(|_reply| ())
    }

    /// One control call, with the ok/error split already decided.
    fn op(&self, request: &serde_json::Value) -> Result<OpReply, Error> {
        let line = self.call(&request.to_string())?;
        let reply: OpReply =
            serde_json::from_str(&line).map_err(|e| Error::Protocol(e.to_string()))?;
        if reply.ok {
            return Ok(reply);
        }
        Err(reply.error.map_or_else(
            || Error::Protocol("not ok, and no error given".to_owned()),
            |e| Error::Refused {
                code: e.code,
                message: e.message,
            },
        ))
    }

    fn call(&self, request: &str) -> Result<String, Error> {
        let stream =
            UnixStream::connect(&self.path).map_err(|e| Error::Unreachable(e.to_string()))?;
        stream
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        stream
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| Error::Unreachable(e.to_string()))?;

        let mut writing = &stream;
        writing
            .write_all(request.as_bytes())
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        writing
            .write_all(b"\n")
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        writing
            .flush()
            .map_err(|e| Error::Unreachable(e.to_string()))?;

        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(|e| Error::Unreachable(e.to_string()))?;
        if line.is_empty() {
            return Err(Error::Unreachable(
                "anchord closed without answering".to_owned(),
            ));
        }
        Ok(line)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;

    /// Captured verbatim from a live anchord (anchor 49e2f9d).
    const REAL: &str = r#"{"ok":true,"sessions":[{
        "args":["-c","echo hi"],"cols":80,"command":"/bin/sh",
        "created_at":1792156042385,"cwd":"/Users/aatsman","env_count":0,
        "exit_code":null,"exited_at":null,"id":"01M52D614H0SSH1WMB0PMA6YAD",
        "labels":{"teleport.kind":"shell","teleport.owner":"teleport"},
        "log_capped_at":null,"login_shell":false,"lost_reason":null,
        "output_bytes":988,"pid":8428,"rows":24,"state":"running"}]}"#;

    #[test]
    fn parses_a_real_reply_and_ignores_fields_we_do_not_use() {
        let parsed: ListReply = serde_json::from_str(REAL).expect("real anchord reply");
        assert!(parsed.ok);
        let s = parsed.sessions.first().expect("one session");
        assert_eq!(s.id, "01M52D614H0SSH1WMB0PMA6YAD");
        assert_eq!(s.lifecycle(), SessionState::Running);
        assert_eq!(s.output_bytes, 988);
        // The key `tally attention` resolves a hook by, so it has to
        // survive the wire and not just `anchor get` (`src/attention.rs`).
        assert_eq!(s.pid, Some(8428));
        assert!(!s.login_shell);
        // teleport's session: not ours, and it claims no pad.
        assert_eq!(s.slot_label(), None);
    }

    #[test]
    fn an_unknown_state_is_lost_rather_than_guessed_at() {
        let s = Session {
            id: "x".to_owned(),
            state: "quantum".to_owned(),
            exit_code: None,
            lost_reason: None,
            labels: BTreeMap::new(),
            output_bytes: 0,
            pid: None,
            cwd: None,
            command: None,
            login_shell: false,
        };
        assert_eq!(s.lifecycle(), SessionState::Lost);
    }

    #[test]
    fn labels_answer_the_three_questions_we_ask_of_them() {
        let mut labels = BTreeMap::new();
        drop(labels.insert(LABEL_SLOT.to_owned(), "work:3:5".to_owned()));
        drop(labels.insert(LABEL_MUTE.to_owned(), "1".to_owned()));
        let s = Session {
            id: "x".to_owned(),
            state: "running".to_owned(),
            exit_code: None,
            lost_reason: None,
            labels,
            output_bytes: 1,
            pid: None,
            cwd: None,
            command: None,
            login_shell: false,
        };
        assert_eq!(s.slot_label(), Some("work:3:5"));
        assert!(s.muted());
        assert!(!s.declared_attention());
    }

    #[test]
    fn an_error_reply_is_not_silently_an_empty_list() {
        let reply = r#"{"ok":false,"error":{"code":"internal","message":"db would not read"}}"#;
        let parsed: ListReply = serde_json::from_str(reply).expect("error reply");
        assert!(!parsed.ok);
        assert_eq!(parsed.sessions.len(), 0);
    }

    #[test]
    fn an_absent_socket_is_unreachable_not_empty() {
        let client = Client::new("/nonexistent/anchor.sock");
        assert!(matches!(client.list(), Err(Error::Unreachable(_))));
    }

    /// Every label we write carries our prefix, so a grep finds all of them.
    #[test]
    fn every_label_is_in_our_namespace() {
        for label in [LABEL_SLOT, LABEL_SPEC, LABEL_MUTE, LABEL_ATTENTION] {
            assert!(label.starts_with(PREFIX), "{label} is outside {PREFIX}");
        }
    }

    /// A socket of our own, named for the test that binds it.
    fn scratch_socket(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("tally-anchor-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(format!("{name}-{}.sock", std::process::id()));
        // A socket file outlives the process that bound it, so a crashed
        // earlier run would make `bind` fail with "address in use".
        let _stale = std::fs::remove_file(&path);
        path
    }

    /// One output frame on the wire, as anchord writes it.
    fn output_frame(offset: u64, bytes: &[u8]) -> Vec<u8> {
        let mut frame = vec![0x01u8];
        frame.extend_from_slice(&offset.to_be_bytes());
        frame.extend_from_slice(
            &u32::try_from(bytes.len())
                .expect("test frame fits")
                .to_be_bytes(),
        );
        frame.extend_from_slice(bytes);
        frame
    }

    /// A header with a length nobody could have meant.
    fn header(tag: u8, offset: u64, len: u32) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        if let Some(slot) = h.first_mut() {
            *slot = tag;
        }
        if let Some(slot) = h.get_mut(1..9) {
            slot.copy_from_slice(&offset.to_be_bytes());
        }
        if let Some(slot) = h.get_mut(9..13) {
            slot.copy_from_slice(&len.to_be_bytes());
        }
        h
    }

    /// The whole point of the cap: a `u32` is 4 GiB wide and we allocate on
    /// it sight unseen.
    #[test]
    fn a_header_claiming_more_than_anchord_can_send_is_refused() {
        assert_eq!(
            decode_header(&header(0x01, 7, 64 * 1024)),
            Some((0x01, 7, 65536))
        );
        let cap = u32::try_from(MAX_FRAME_BYTES).expect("the cap fits a u32");
        assert!(
            decode_header(&header(0x01, 0, cap)).is_some(),
            "a full replay round is legitimate"
        );
        assert_eq!(decode_header(&header(0x01, 0, cap + 1)), None);
        assert_eq!(decode_header(&header(0x01, 0, u32::MAX)), None);
    }

    /// Refused has to mean *ended*, not panicked and not truncated: `None`
    /// is the ending the supervisor already handles.
    #[test]
    fn an_oversized_header_ends_the_stream_instead_of_allocating() {
        let (mut daemon, ours) = UnixStream::pair().expect("socketpair");
        daemon
            .write_all(&header(0x01, 0, u32::MAX))
            .expect("write a bogus header");
        let mut frames = Frames {
            reader: BufReader::new(ours),
            offset: 0,
        };
        assert_eq!(frames.next_frame(), None);
    }

    /// One PTY chunk is the common case and must still go through whole.
    #[test]
    fn a_full_pty_chunk_is_delivered_untruncated() {
        let (mut daemon, ours) = UnixStream::pair().expect("socketpair");
        let bytes = vec![b'x'; 64 * 1024];
        let written = std::thread::spawn(move || {
            daemon
                .write_all(&output_frame(99, &bytes))
                .expect("write a chunk");
        });
        let mut frames = Frames {
            reader: BufReader::new(ours),
            offset: 0,
        };
        match frames.next_frame() {
            Some(Frame::Output { offset, bytes }) => {
                assert_eq!(offset, 99);
                assert_eq!(bytes.len(), 64 * 1024);
            }
            other => panic!("expected one output frame, got {other:?}"),
        }
        written.join().expect("writer");
    }

    /// A daemon that accepts and never answers is the shape that parks a
    /// watcher forever, and a parked watcher holds its session's slot in
    /// `Watchers::watching` for the life of the process -- so that pad goes
    /// deaf and no reconcile ever replaces it.
    #[test]
    fn an_attach_that_is_never_answered_gives_up_rather_than_parking_forever() {
        let path = scratch_socket("silent");
        let listener = UnixListener::bind(&path).expect("bind");
        // Hold the accepted connection open for the duration. Dropping it
        // would close the socket and make this an EOF test, which passes
        // with or without a timeout.
        let server = std::thread::spawn(move || listener.accept().map(|(stream, _addr)| stream));

        let client = Client::new(&path);
        let started = std::time::Instant::now();
        let result = client.attach("01M52D614H0SSH1WMB0PMA6YAD", 0);
        assert!(
            matches!(result, Err(Error::Unreachable(_))),
            "a silent daemon must end the attach, not park on it"
        );
        assert!(
            started.elapsed() < TIMEOUT * 4,
            "the attach must give up on the order of the timeout, not eventually"
        );
        drop(server.join().expect("server thread"));
        let _gone = std::fs::remove_file(&path);
    }

    /// The other half of the same change: the handshake bound must not
    /// survive into the stream. An agent waiting on a human says nothing
    /// for hours, and that is the session the surface exists to show --
    /// dropping its watcher every two seconds would be the new bug.
    #[test]
    fn a_quiet_session_is_not_dropped_by_the_handshake_timeout() {
        let path = scratch_socket("quiet");
        let listener = UnixListener::bind(&path).expect("bind");
        let server = std::thread::spawn(move || {
            let (mut stream, _addr) = listener.accept().expect("accept");
            let mut request = String::new();
            BufReader::new(stream.try_clone().expect("clone"))
                .read_line(&mut request)
                .expect("read the attach request");
            stream
                .write_all(b"{\"id\":1,\"ok\":true,\"offset\":0}\n")
                .expect("write the attach reply");
            // Quiet for longer than the handshake bound, then speak.
            std::thread::sleep(TIMEOUT + Duration::from_millis(500));
            stream
                .write_all(&output_frame(0, b"ding"))
                .expect("write a frame");
        });

        let client = Client::new(&path);
        let mut frames = client
            .attach("01M52D614H0SSH1WMB0PMA6YAD", 0)
            .expect("attach");
        assert_eq!(
            frames.next_frame(),
            Some(Frame::Output {
                offset: 0,
                bytes: b"ding".to_vec(),
            }),
            "a frame after a long silence must still arrive"
        );
        server.join().expect("server thread");
        let _gone = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod path_tests {
    use super::Client;

    /// `VAR=` is not `VAR=/path`. Taking an empty value literally turns an
    /// absolute path into a relative one, and the symptom is "anchord
    /// unreachable" with no clue why.
    #[test]
    fn an_empty_env_var_is_not_a_path() {
        assert!(
            Client::default_path().is_absolute(),
            "default socket path must be absolute"
        );
        assert!(crate::env_path("TALLY_DEFINITELY_UNSET_VAR").is_none());
    }
}
