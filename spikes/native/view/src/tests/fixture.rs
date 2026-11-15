//! The page's fixtures as Rust values: its short ids ("3f2a") become
//! session ids that start with them, so a test can name a session the way
//! the page's test does.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use vessel_proto::{
    HarnessKind, NodeId, SessionId, SessionInfo, SessionMode, SessionSpec, SessionStatus,
};

use crate::{Card, Fleet, Machine, Reach};

/// The page's time `text`.
pub(super) fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

/// The session id that starts with the page's short `id` (hex, up to 8).
pub(super) fn id(short: &str) -> SessionId {
    format!("{short:0<8}-0000-4000-8000-000000000000")
        .parse()
        .unwrap()
}

/// The page's `id` back from a session id (its first `len` characters).
pub(super) fn short(card: &Card, len: usize) -> String {
    card.session.to_string().chars().take(len).collect()
}

/// A structured Claude Code session `id`, created at `when`.
pub(super) fn session(short: &str, when: &str, status: SessionStatus) -> SessionInfo {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(id(short), spec, at(when));
    info.status = status;
    info
}

/// The same, in the folder `cwd`.
pub(super) fn in_folder(mut info: SessionInfo, cwd: &str) -> SessionInfo {
    info.spec.cwd = Some(PathBuf::from(cwd));
    info
}

/// Started by the agent of the session `parent`.
pub(super) fn agent_of(mut info: SessionInfo, parent: &str) -> SessionInfo {
    info.created_by = vessel_proto::Creator::Agent {
        node: NodeId::random(),
        session: id(parent),
    };
    info
}

/// A machine named `name` with `sessions`, connected or not.
pub(super) fn machine(name: &str, online: bool, sessions: &[SessionInfo]) -> Machine {
    Machine {
        node: None,
        connection_epoch: None,
        name: name.to_owned(),
        reach: if online {
            Reach::Online
        } else {
            Reach::Unreachable("offline".to_owned())
        },
        sessions: sessions
            .iter()
            .map(|session| Card::from_session(name, session))
            .collect(),
        agents: Vec::new(),
        protocol: online.then_some(vessel_proto::PROTOCOL_VERSION),
        version: None,
    }
}

/// A book of `machines`.
pub(super) fn fleet(machines: Vec<Machine>) -> Fleet {
    Fleet {
        machines,
        read_at: None,
    }
}
