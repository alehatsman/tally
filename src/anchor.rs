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

/// How long to wait on anchord before calling the tick failed.
const TIMEOUT: Duration = Duration::from_secs(2);

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
        // u8 tag, u64 offset BE, u32 length BE. The exit payload inside is
        // little-endian -- mixed, deliberately, and only the code says so.
        let mut header = [0u8; 13];
        self.reader.read_exact(&mut header).ok()?;
        let tag = *header.first()?;
        let offset = u64::from_be_bytes(header.get(1..9)?.try_into().ok()?);
        let len = u32::from_be_bytes(header.get(9..13)?.try_into().ok()?);
        let mut payload = vec![0u8; usize::try_from(len).ok()?];
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
        reply.session.ok_or_else(|| {
            Error::Protocol("create said ok and named no session".to_owned())
        })
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
