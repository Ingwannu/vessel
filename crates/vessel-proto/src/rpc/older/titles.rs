//! Titles longer than an older peer decodes (TRACKER 801, amending ADR
//! 0022) in the replies that carry one outside a session: the inbox, the
//! terminals (script runs included: `script.list`'s runs and a started
//! run, which `terminal.rename` can name) and the workspace registry. Each is cut by
//! [`SessionTitle::for_protocol`]; sessions, their templates and the
//! `renamed` event are cut where they are projected already
//! (`SessionInfo::for_protocol`, `templates`, `SessionEvent`).
//!
//! [Decision Log]
//! - Purpose: a peer before `LONG_TITLES_PROTOCOL` parses every title it
//!   decodes at 120 characters, so one longer title would fail its whole
//!   frame and drop its connection.
//! - Choice: one rewrite per reply type, as every other older-peer rule is
//!   written (`Reply::for_protocol`), returning `None` when nothing is
//!   longer, so a current peer costs no copy. A list is copied from its
//!   first changed item (`templates::changed`).

use super::templates::changed;
use crate::inbox::{InboxEntry, InboxPage};
use crate::registry::{RegistrySnapshot, WorkspaceDescriptor};
use crate::scripts::{ScriptList, ScriptRunOutcome};
use crate::terminals::{TerminalInfo, TerminalList, TerminalSubscribed};
use crate::{ProtocolVersion, SessionTitle};

/// An inbox entry with its session's title cut.
pub(super) fn inbox_entry(entry: &InboxEntry, protocol: ProtocolVersion) -> Option<InboxEntry> {
    let title = SessionTitle::optional_for_protocol(entry.title.as_ref(), protocol)?;
    let mut older = entry.clone();
    older.title = title;
    Some(older)
}

/// A page of the inbox, each entry as [`inbox_entry`].
pub(super) fn inbox_page(page: &InboxPage, protocol: ProtocolVersion) -> Option<InboxPage> {
    let entries = changed(&page.entries, |entry| inbox_entry(entry, protocol))?;
    let mut older = page.clone();
    older.entries = entries;
    Some(older)
}

/// A terminal with the title a person gave it cut.
pub(super) fn terminal(info: &TerminalInfo, protocol: ProtocolVersion) -> Option<TerminalInfo> {
    let title = SessionTitle::optional_for_protocol(info.title.as_ref(), protocol)?;
    let mut older = info.clone();
    older.title = title;
    Some(older)
}

/// The terminals, each as [`terminal`].
pub(super) fn terminals(list: &TerminalList, protocol: ProtocolVersion) -> Option<TerminalList> {
    let terminals = changed(&list.terminals, |info| terminal(info, protocol))?;
    let mut older = list.clone();
    older.terminals = terminals;
    Some(older)
}

/// A terminal subscription's opening state, as [`terminal`].
pub(super) fn subscribed(
    subscribed: &TerminalSubscribed,
    protocol: ProtocolVersion,
) -> Option<TerminalSubscribed> {
    let info = terminal(&subscribed.terminal, protocol)?;
    let mut older = subscribed.clone();
    older.terminal = info;
    Some(older)
}

/// A project's scripts: its commands' rule first, then its runs' titles.
pub(super) fn script_list(list: &ScriptList, protocol: ProtocolVersion) -> Option<ScriptList> {
    let commands = list.commands.for_protocol(protocol);
    let runs = changed(&list.runs, |info| terminal(info, protocol));
    if commands.is_none() && runs.is_none() {
        return None;
    }
    let mut older = list.clone();
    if let Some(commands) = commands {
        older.commands = commands;
    }
    if let Some(runs) = runs {
        older.runs = runs;
    }
    Some(older)
}

/// A script's run or restart: a started run's title cut, any other outcome
/// by its own rule (an approval it could not show).
pub(super) fn script_outcome(
    outcome: &ScriptRunOutcome,
    protocol: ProtocolVersion,
) -> Option<ScriptRunOutcome> {
    if let ScriptRunOutcome::Started { terminal: run } = outcome {
        return terminal(run, protocol).map(|run| ScriptRunOutcome::Started {
            terminal: Box::new(run),
        });
    }
    outcome.for_protocol(protocol)
}

/// A workspace with its given title cut.
pub(super) fn workspace(
    descriptor: &WorkspaceDescriptor,
    protocol: ProtocolVersion,
) -> Option<WorkspaceDescriptor> {
    let title = SessionTitle::optional_for_protocol(descriptor.title.as_ref(), protocol)?;
    let mut older = descriptor.clone();
    older.title = title;
    Some(older)
}

/// The workspaces, each as [`workspace`].
pub(super) fn workspaces(
    list: &[WorkspaceDescriptor],
    protocol: ProtocolVersion,
) -> Option<Vec<WorkspaceDescriptor>> {
    changed(list, |descriptor| workspace(descriptor, protocol))
}

/// The registry snapshot: its label rule first, then its workspaces'
/// titles.
pub(super) fn registry(
    snapshot: &RegistrySnapshot,
    protocol: ProtocolVersion,
) -> Option<RegistrySnapshot> {
    let projected = snapshot.for_protocol(protocol);
    let source = projected.as_ref().unwrap_or(snapshot);
    let Some(cut) = workspaces(&source.workspaces, protocol) else {
        return projected;
    };
    let mut older = projected.unwrap_or_else(|| snapshot.clone());
    older.workspaces = cut;
    Some(older)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use serde::Deserialize;

    use super::*;
    use crate::event::{EventRecord, SessionEvent};
    use crate::harness::HarnessKind;
    use crate::id::{EventSeq, SessionId};
    use crate::rpc::Reply;
    use crate::session::{SessionInfo, SessionMode, SessionSpec};
    use crate::title::TitleOrigin;

    /// A title as a peer before `LONG_TITLES_PROTOCOL` decodes one: its
    /// parse refuses more than 120 characters.
    #[derive(Debug)]
    struct OlderTitle(String);

    impl<'de> Deserialize<'de> for OlderTitle {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let text = String::deserialize(deserializer)?;
            if text.chars().count() > SessionTitle::LEGACY_MAX_CHARS {
                return Err(serde::de::Error::custom(
                    "a title is at most 120 characters",
                ));
            }
            Ok(Self(text))
        }
    }

    /// What an older peer reads of a session: its titles, by that parse.
    #[derive(Debug, Deserialize)]
    struct OlderSession {
        title: Option<OlderTitle>,
        spec: OlderSpec,
    }

    #[derive(Debug, Deserialize)]
    struct OlderSpec {
        title: Option<OlderTitle>,
    }

    fn older() -> ProtocolVersion {
        let since = crate::LONG_TITLES_PROTOCOL.unwrap();
        ProtocolVersion::new(since.get().saturating_sub(1))
    }

    fn long() -> SessionTitle {
        format!("{} 끝", "가".repeat(148)).parse().unwrap()
    }

    fn session() -> SessionInfo {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_title(long());
        let mut info = SessionInfo::new(SessionId::random(), spec, DateTime::UNIX_EPOCH);
        info.title = Some(long());
        info
    }

    #[test]
    fn an_older_peer_decodes_a_session_that_holds_a_150_character_title() {
        let info = session();
        assert_eq!(long().as_str().chars().count(), 150);
        let whole = serde_json::to_string(&info).unwrap();
        assert!(
            serde_json::from_str::<OlderSession>(&whole).is_err(),
            "unprojected, the older parse refuses the frame"
        );

        let reply = Reply::ListSessions(vec![info.clone()]);
        let Some(Reply::ListSessions(sessions)) = reply.for_protocol(older()) else {
            panic!("the list is rewritten for an older peer");
        };
        let projected = serde_json::to_string(&sessions[0]).unwrap();
        let read: OlderSession = serde_json::from_str(&projected).unwrap();
        let title = read.title.unwrap().0;
        assert_eq!(title.chars().count(), SessionTitle::LEGACY_MAX_CHARS);
        assert!(
            long().as_str().starts_with(&title),
            "its first 120 characters"
        );
        assert_eq!(read.spec.title.unwrap().0, title);

        // A current peer reads both titles whole (other rules may still
        // rewrite the session for it).
        let current = crate::LONG_TITLES_PROTOCOL.unwrap();
        let read = info.for_protocol(current).unwrap_or_else(|| info.clone());
        assert_eq!(read.title, Some(long()));
        assert_eq!(read.spec.title, Some(long()));
    }

    #[test]
    fn a_rename_event_reaches_an_older_peer_cut() {
        let record = EventRecord {
            seq: EventSeq::ZERO,
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Renamed {
                title: long(),
                origin: TitleOrigin::Typed,
            },
        };
        let projected = record.for_protocol(older()).unwrap();
        let SessionEvent::Renamed { title, .. } = &projected.event else {
            panic!("still a rename");
        };
        assert_eq!(
            title.as_str().chars().count(),
            SessionTitle::LEGACY_MAX_CHARS
        );
    }

    #[test]
    fn a_terminal_reaches_an_older_peer_cut() {
        let cut =
            |title: Option<&SessionTitle>| title.map_or(0, |title| title.as_str().chars().count());
        let mut info = TerminalInfo::new(
            crate::TerminalId::random(),
            crate::terminals::TerminalPlace::Project {
                project: crate::ProjectId::random(),
                dir: None,
            },
            "checkout".into(),
            crate::terminals::TerminalKind::Shell {
                program: "/bin/sh".into(),
            },
            DateTime::UNIX_EPOCH,
        );
        info.title = Some(long());
        let older_terminal = terminal(&info, older()).unwrap();
        assert_eq!(
            cut(older_terminal.title.as_ref()),
            SessionTitle::LEGACY_MAX_CHARS
        );
        assert_eq!(terminal(&info, crate::PROTOCOL_VERSION), None);
        let list = TerminalList {
            terminals: vec![info],
        };
        assert!(terminals(&list, older()).is_some());
    }

    /// What an older peer reads of a script run: its title, by that parse.
    #[derive(Debug, Deserialize)]
    struct OlderRun {
        title: Option<OlderTitle>,
    }

    fn run() -> TerminalInfo {
        let mut info = TerminalInfo::new(
            crate::TerminalId::random(),
            crate::terminals::TerminalPlace::Project {
                project: crate::ProjectId::random(),
                dir: None,
            },
            "checkout".into(),
            crate::terminals::TerminalKind::Shell {
                program: "/bin/sh".into(),
            },
            DateTime::UNIX_EPOCH,
        );
        // `terminal.rename` names a script's run as any terminal.
        info.title = Some(long());
        info
    }

    #[test]
    fn script_runs_reach_an_older_peer_cut() {
        let list = Reply::ListScripts(ScriptList {
            root: "/work/app".into(),
            commands: crate::workspace::ProjectCommands::None,
            runs: vec![run()],
        });
        let whole = serde_json::to_value(&list).unwrap();
        assert!(serde_json::from_value::<Vec<OlderRun>>(whole["runs"].clone()).is_err());
        let Some(Reply::ListScripts(listed)) = list.for_protocol(older()) else {
            panic!("the runs are rewritten for an older peer");
        };
        let value = serde_json::to_value(&listed).unwrap();
        let runs: Vec<OlderRun> = serde_json::from_value(value["runs"].clone()).unwrap();
        let title = runs[0].title.as_ref().unwrap();
        assert_eq!(title.0.chars().count(), SessionTitle::LEGACY_MAX_CHARS);

        for reply in [
            Reply::RunScript(ScriptRunOutcome::Started {
                terminal: Box::new(run()),
            }),
            Reply::RestartScript(ScriptRunOutcome::Started {
                terminal: Box::new(run()),
            }),
        ] {
            let projected = reply.for_protocol(older()).unwrap();
            let (Reply::RunScript(outcome) | Reply::RestartScript(outcome)) = &projected else {
                panic!("still a script outcome");
            };
            let value = serde_json::to_value(outcome).unwrap();
            assert_eq!(value["outcome"], "started");
            let started: OlderRun = serde_json::from_value(value["terminal"].clone()).unwrap();
            assert_eq!(
                started.title.unwrap().0.chars().count(),
                SessionTitle::LEGACY_MAX_CHARS
            );
            assert_eq!(reply.for_protocol(crate::PROTOCOL_VERSION), None);
        }
    }
}
