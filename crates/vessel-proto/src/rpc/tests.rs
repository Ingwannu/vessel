use chrono::DateTime;

use super::*;
use crate::agent::AgentEvent;
use crate::conversation::PromptSession;
use crate::event::{EventRecord, SessionEvent};
use crate::harness::HarnessKind;
use crate::id::{DeviceId, EventSeq, RequestId, SubscriptionId};
use crate::session::{SessionMode, SessionSpec};
use crate::terminal::TerminalSize;
use crate::{Deliver, DeviceInfo, Forward, InviteDevice, RelayCall, SessionInfo, SetSessionOption};

#[test]
fn unit_requests_round_trip() {
    let frame = ClientFrame::Request {
        id: RequestId::new(7),
        call: GetNode.into_call(),
    };
    let json = serde_json::to_string(&frame).unwrap();
    assert_eq!(
        json,
        r#"{"type":"request","id":7,"call":{"method":"node.info","params":null}}"#
    );
    assert_eq!(serde_json::from_str::<ClientFrame>(&json).unwrap(), frame);
}

#[test]
fn every_method_declares_authority_and_admin_is_rare() {
    let admin: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.scope() == Some(Scope::Admin))
        .map(|method| method.as_str())
        .collect();
    assert_eq!(
        admin,
        [
            "node.stop",
            "device.invite",
            "device.list",
            "device.revoke",
            "profile.reload",
            "fleet.join",
            "fleet.leave",
            "fleet.sync",
            "fleet.published",
            "fleet.members",
            "fleet.forget",
            "memory.export",
            "memory.import",
            "secret.set",
            "secret.list",
            "secret.delete",
            "secret.copy",
            "agent.add",
            "agent.remove",
            "agent.install",
            "agent.install_output",
            "usage.lookup",
            "node.set_settings",
            "terminal_profile.get",
            "terminal_profile.set"
        ]
    );
    assert_eq!(
        Method::SendInput
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Operate
    );
    assert_eq!(
        Method::ReadEvents
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Read
    );
}

#[test]
fn replies_bind_to_their_request() {
    let reply = Reply::SendInput(Ack);
    assert_eq!(SendInput::response_from(reply.clone()), Ok(Ack));
    assert_eq!(
        KillSession::response_from(reply),
        Err(UnexpectedReply {
            expected: Method::KillSession,
            got: Method::SendInput,
        })
    );
}

const V4: ProtocolVersion = ProtocolVersion::new(4);
const V5: ProtocolVersion = ProtocolVersion::new(5);
const V6: ProtocolVersion = ProtocolVersion::new(6);
const V8: ProtocolVersion = ProtocolVersion::new(8);
const V9: ProtocolVersion = ProtocolVersion::new(9);
const V10: ProtocolVersion = ProtocolVersion::new(10);

#[test]
fn every_method_knows_its_first_version() {
    assert_eq!(Method::NodeInfo.since(), Some(ProtocolVersion::new(2)));
    assert_eq!(Method::StopNode.since(), Some(ProtocolVersion::new(3)));
    assert_eq!(Method::RevokeDevice.since(), Some(V4));
    let v5_fleet = [
        Method::GetManifest,
        Method::ReadBlob,
        Method::WatchFleet,
        Method::ReportApplied,
        Method::ListMembers,
    ];
    let added_fleet = [
        Method::JoinFleet,
        Method::LeaveFleet,
        Method::SyncFleet,
        Method::GetFleetStatus,
    ];
    // N3's fleet controls are 15 on the node line: preview's 13 and the
    // fork's 14 were published before them (`PROTOCOL_FIFTEEN` below).
    for method in added_fleet {
        assert_eq!(method.since(), Some(ProtocolVersion::new(15)));
    }
    assert_eq!(Method::GetPublished.since(), Some(ProtocolVersion::new(16)));
    for method in Method::ALL {
        assert!(
            method.since() <= Some(crate::PROTOCOL_VERSION),
            "{method:?}"
        );
        let name = method.as_str();
        let fleet = name.starts_with("fleet.")
            && !added_fleet.contains(method)
            && *method != Method::GetPublished;
        let profile = name.starts_with("profile.") || *method == Method::MoveProfile;
        let v6 = (fleet && !v5_fleet.contains(method))
            || ["session.export", "session.import"].contains(&name)
            || ["memory.", "transfer.", "secret."]
                .iter()
                .any(|prefix| name.starts_with(prefix));
        assert_eq!(method.since() == Some(V6), v6, "{method:?}");
        assert_eq!(
            method.since() == Some(V5),
            (fleet || profile) && !v6,
            "{method:?}"
        );
    }
    // Protocol 9: what a session runs with, and titles.
    let v9: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(V9))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(
        v9,
        [
            ("harness.options", Scope::Read),
            ("session.options", Scope::Read),
            ("session.set_option", Scope::Operate),
            ("session.rename", Scope::Operate),
        ]
    );
    protocol_ten_workspace_methods_keep_their_version_and_scope();
}

/// Protocol 10, track B: projects, worktrees, git and files. Reads are
/// `read`; everything that changes a folder or a repository is `operate`;
/// nothing needs `admin`. Those methods are listed (`PROTOCOL_TEN_WORKSPACE`);
/// a later method in the same families is pending until the integrator
/// assigns its number, and its package's own test then names it in
/// `WORKSPACE_FAMILIES_LATER`.
fn protocol_ten_workspace_methods_keep_their_version_and_scope() {
    for method in Method::ALL {
        let name = method.as_str();
        let workspace = ["project.", "worktree.", "git.", "file."]
            .iter()
            .any(|prefix| name.starts_with(prefix));
        if !workspace || WORKSPACE_FAMILIES_LATER.contains(&name) {
            continue;
        }
        if !PROTOCOL_TEN_WORKSPACE.contains(&name) {
            assert_eq!(
                method.since(),
                None,
                "{method:?} is new: pending until assigned"
            );
            continue;
        }
        assert_eq!(method.since(), Some(V10), "{method:?}");
        let reads = [
            "project.list",
            "project.get",
            "project.commands",
            "worktree.list",
            "worktree.output",
            "worktree.snapshots",
            "git.status",
            "git.changes",
            "git.diff",
            "git.show",
            "git.branches",
            "git.log",
            "git.stashes",
            "file.list",
            "file.read",
            "file.find",
            "file.grep",
        ];
        let expected = if reads.contains(&name) {
            Scope::Read
        } else {
            Scope::Operate
        };
        assert_eq!(
            method.scope().expect("legacy method requires a scope"),
            expected,
            "{method:?}"
        );
    }
}

/// The project, worktree, git and file methods protocol 10 brought (track
/// B), exactly; their version and scope are checked above.
const PROTOCOL_TEN_WORKSPACE: &[&str] = &[
    "project.list",
    "project.get",
    "project.add",
    "project.clone",
    "project.init",
    "project.remove",
    "project.commands",
    "project.approve",
    "worktree.list",
    "worktree.create",
    "worktree.output",
    "worktree.setup",
    "worktree.remove",
    "worktree.snapshots",
    "worktree.drop_snapshot",
    "git.status",
    "git.changes",
    "git.diff",
    "git.show",
    "git.branches",
    "git.log",
    "git.stashes",
    "git.stage",
    "git.unstage",
    "git.discard",
    "git.commit",
    "git.fetch",
    "git.push",
    "git.pull",
    "git.switch",
    "git.rename_branch",
    "git.stash",
    "file.list",
    "file.read",
    "file.write",
    "file.create_dir",
    "file.rename",
    "file.delete",
    "file.find",
    "file.grep",
];

/// Methods in the same families added after protocol 10 and assigned a
/// number: each is checked by its package's own test, which adds it here.
const WORKSPACE_FAMILIES_LATER: &[&str] = &[];

/// Every protocol 11 method and its scope, by track: B2's pull requests
/// (ADR 0029) and C's terminals, scripts and ports (ADR 0031) read with
/// `read` and act with `operate`; G's agents (ADR 0032) are read with
/// `read` and added, removed and installed with `admin`; D's schedules and
/// saved prompts (ADR 0033) are read with `read` and written with
/// `operate`. Written out, so a method added to protocol 11 fails here
/// until its scope is decided.
const PROTOCOL_ELEVEN: &[(&str, Scope)] = &[
    ("forge.status", Scope::Read),
    ("pr.list", Scope::Read),
    ("pr.get", Scope::Read),
    ("pr.open", Scope::Operate),
    ("pr.merge", Scope::Operate),
    ("pr.feedback", Scope::Operate),
    ("terminal.create", Scope::Operate),
    ("terminal.list", Scope::Read),
    ("terminal.get", Scope::Read),
    ("terminal.read", Scope::Read),
    ("terminal.subscribe", Scope::Read),
    ("terminal.send", Scope::Operate),
    ("terminal.rename", Scope::Operate),
    ("terminal.kill", Scope::Operate),
    ("terminal.close", Scope::Operate),
    ("script.list", Scope::Read),
    ("script.run", Scope::Operate),
    ("script.restart", Scope::Operate),
    ("port.list", Scope::Read),
    ("agent.list", Scope::Read),
    ("agent.add", Scope::Admin),
    ("agent.remove", Scope::Admin),
    ("agent.status", Scope::Read),
    ("agent.catalogue", Scope::Read),
    ("agent.install", Scope::Admin),
    ("agent.install_output", Scope::Admin),
    ("schedule.list", Scope::Read),
    ("schedule.get", Scope::Read),
    ("schedule.create", Scope::Operate),
    ("schedule.update", Scope::Operate),
    ("schedule.pause", Scope::Operate),
    ("schedule.resume", Scope::Operate),
    ("schedule.run", Scope::Operate),
    ("schedule.delete", Scope::Operate),
    ("schedule.runs", Scope::Read),
    ("prompt.list", Scope::Read),
    ("prompt.create", Scope::Operate),
    ("prompt.update", Scope::Operate),
    ("prompt.delete", Scope::Operate),
];

#[test]
fn protocol_eleven_methods_know_their_version_and_scope() {
    let eleven: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(ProtocolVersion::new(11)))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(eleven, PROTOCOL_ELEVEN);
}

/// Protocol 12's methods (milestone 11, track A2), in table order.
const PROTOCOL_TWELVE: &[(&str, Scope)] = &[
    ("session.search", Scope::Read),
    ("session.park", Scope::Operate),
    ("session.wake", Scope::Operate),
    ("node.settings", Scope::Read),
    ("node.set_settings", Scope::Admin),
];

#[test]
fn protocol_twelve_methods_know_their_version_and_scope() {
    let twelve: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(ProtocolVersion::new(12)))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(twelve, PROTOCOL_TWELVE);
}

/// Protocol 14's methods (milestone 11's track A2, ADR 0052), pinned as
/// the other versions' are. Protocol 13 (N3's preview) is pinned in
/// `automation::preview`'s own test.
const PROTOCOL_FOURTEEN: &[(&str, Scope)] = &[
    ("session.fork", Scope::Operate),
    ("session.rewind", Scope::Operate),
];

#[test]
fn protocol_fourteen_methods_know_their_version_and_scope() {
    let fourteen: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(ProtocolVersion::new(14)))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(fourteen, PROTOCOL_FOURTEEN);
}

/// Protocol 15's methods (milestone 12's track N3, ADR 0044): fleet
/// membership over the wire, numbered after the versions already published.
const PROTOCOL_FIFTEEN: &[(&str, Scope)] = &[
    ("fleet.join", Scope::Admin),
    ("fleet.leave", Scope::Admin),
    ("fleet.sync", Scope::Admin),
    ("fleet.status", Scope::Read),
];

#[test]
fn protocol_fifteen_methods_know_their_version_and_scope() {
    let fifteen: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(ProtocolVersion::new(15)))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(fifteen, PROTOCOL_FIFTEEN);
}

/// Protocol 16's method (ADR 0056): what a machine shares, for a person.
const PROTOCOL_SIXTEEN: &[(&str, Scope)] = &[("fleet.published", Scope::Admin)];

#[test]
fn protocol_sixteen_methods_know_their_version_and_scope() {
    let sixteen: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(ProtocolVersion::new(16)))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(sixteen, PROTOCOL_SIXTEEN);
}

/// Protocol 17's method (track A2's slice 4, ADR 0053): an agent's own
/// history, written at 14 and numbered after 15 and 16 on the node line.
const PROTOCOL_SEVENTEEN: &[(&str, Scope)] = &[("agent.history", Scope::Operate)];

#[test]
fn protocol_seventeen_methods_know_their_version_and_scope() {
    let seventeen: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == Some(ProtocolVersion::new(17)))
        .map(|method| {
            (
                method.as_str(),
                method.scope().expect("legacy method requires a scope"),
            )
        })
        .collect();
    assert_eq!(seventeen, PROTOCOL_SEVENTEEN);
}

#[test]
fn every_hop_of_the_envelope_has_its_own_scope() {
    // What a member holds on its home: `sync` for the bundle, `relay` only
    // when its owner asked; `relay` opens nothing but passing calls on.
    for method in Method::ALL {
        assert_eq!(
            method.scope() == Some(Scope::Relay),
            *method == Method::Relay,
            "{method:?}"
        );
    }
    assert_eq!(
        Method::Forward
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Read
    );
    assert_eq!(
        Method::Deliver
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Read
    );
    // Every secret row is `admin`, and no row returns a value.
    for method in [
        Method::SetSecret,
        Method::ListSecrets,
        Method::DeleteSecret,
        Method::CopySecret,
    ] {
        assert_eq!(
            method.scope().expect("legacy method requires a scope"),
            Scope::Admin,
            "{method:?}"
        );
    }
    // Memory is kept per account on purpose: moving it is `admin`.
    assert_eq!(
        Method::ExportMemory
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Admin
    );
    assert_eq!(
        Method::ImportMemory
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Admin
    );
    assert_eq!(
        Method::ExportSession
            .scope()
            .expect("legacy method requires a scope"),
        Scope::Operate
    );
}

/// What other nodes may ask here (`Method::forwardable` says why each
/// group is in or out): changing it is a decision, so it is written out.
const FORWARDABLE: &[&str] = &[
    "node.info",
    "harness.list",
    "session.list",
    "session.get",
    "session.create",
    "session.side_open",
    "session.side_prompt",
    "session.side_reset",
    "session.send",
    "session.kill",
    "session.read",
    "device.self",
    "profile.list",
    "session.move_profile",
    "fleet.status",
    "session.export",
    "session.import",
    "memory.export",
    "memory.import",
    "transfer.read",
    "transfer.close",
    "secret.set",
    "secret.list",
    "lsp.workspace_symbols",
    "lsp.document_symbols",
    "lsp.definition",
    "lsp.references",
    "lsp.hover",
    "lsp.implementations",
    "lsp.type_definition",
    "lsp.incoming_calls",
    "lsp.outgoing_calls",
    "lsp.diagnostics",
    "model.list",
    "harness.options",
    "session.options",
    "session.set_option",
    "session.rename",
    "session.prompt",
    "session.queue",
    "session.conversation",
    "session.resume",
    "session.mark",
    "session.delete",
    "attachment.begin",
    "attachment.write",
    "attachment.finish",
    "attachment.read",
    "project.list",
    "project.get",
    "project.add",
    "project.clone",
    "project.init",
    "project.remove",
    "project.commands",
    "project.approve",
    "worktree.list",
    "worktree.create",
    "worktree.output",
    "worktree.setup",
    "worktree.remove",
    "worktree.snapshots",
    "worktree.drop_snapshot",
    "git.status",
    "git.changes",
    "git.diff",
    "git.show",
    "git.branches",
    "git.log",
    "git.stashes",
    "git.stage",
    "git.unstage",
    "git.discard",
    "git.commit",
    "git.fetch",
    "git.push",
    "git.pull",
    "git.switch",
    "git.rename_branch",
    "git.stash",
    "file.list",
    "file.read",
    "file.write",
    "file.create_dir",
    "file.rename",
    "file.delete",
    "file.find",
    "file.grep",
    "forge.status",
    "pr.list",
    "pr.get",
    "pr.open",
    "pr.merge",
    "pr.feedback",
    "terminal.create",
    "terminal.list",
    "terminal.get",
    "terminal.read",
    "terminal.send",
    "terminal.rename",
    "terminal.kill",
    "terminal.close",
    "script.list",
    "script.run",
    "script.restart",
    "port.list",
    "agent.list",
    "agent.status",
    "usage.query",
    // The inbox reads/marks forward; device-scoped push calls never do.
    "inbox.list",
    "inbox.get",
    "inbox.mark",
    "usage.limits",
    "usage.lookup",
    "schedule.preview",
    "schedule.list",
    "schedule.get",
    "schedule.create",
    "schedule.update",
    "schedule.pause",
    "schedule.resume",
    "schedule.run",
    "schedule.delete",
    "schedule.runs",
    "prompt.list",
    "prompt.create",
    "prompt.update",
    "prompt.delete",
    "session.search",
    "session.park",
    "session.wake",
    "session.fork",
    "session.rewind",
    "agent.history",
    "session.detach",
    "workspace.registry",
    "workspace.list",
    "workspace.get",
    "workspace.rename",
    "workspace.pin",
    "workspace.release_automation",
    "git.compare",
    "git.compare_file",
    "creation.observe",
    "creation.subscribe",
    "terminal_profile.list",
];

#[test]
fn the_forwardable_set_is_pinned() {
    let forwardable: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.forwardable())
        .map(|method| method.as_str())
        .collect();
    assert_eq!(forwardable, FORWARDABLE);
}

#[test]
fn a_forwarded_call_nests_its_inner_call() {
    let inner = ReadEvents {
        session: crate::id::SessionId::random(),
        from: EventSeq::ZERO,
        limit: 10,
        wait: Some(30),
    }
    .into_call();
    let call = Forward {
        node: crate::id::NodeId::random(),
        call: Box::new(inner.clone()),
    }
    .into_call();
    let json = serde_json::to_string(&call).unwrap();
    assert!(
        json.starts_with(r#"{"method":"fleet.forward","params":{"node":""#),
        "{json}"
    );
    assert!(
        json.contains(r#""call":{"method":"session.read""#),
        "{json}"
    );
    assert_eq!(serde_json::from_str::<Call>(&json).unwrap(), call);
    // Both the envelope and this inner request need 6. Every typed hop
    // must know an inner method and its fields before re-encoding them,
    // and each connection checks the maximum requirement before dispatch.
    assert_eq!(call.since(), Some(V6));
    assert_eq!(inner.since(), Some(V6));
}

/// Protocol 18's drivers (ADR 0057): a settings change naming them needs
/// 18, where an older node would ignore them and keep the agents on ACP;
/// one that does not, 12.
#[test]
fn a_settings_change_naming_drivers_needs_protocol_eighteen() {
    use crate::settings::{AgentDriver, SetAgentDrivers, SetNodeSettings};
    let parking = SetNodeSettings {
        park_after: Some(crate::parking::ParkAfter::Never),
        drivers: None,
    };
    assert_eq!(parking.into_call().since(), Some(ProtocolVersion::new(12)));
    let drivers = SetNodeSettings {
        park_after: None,
        drivers: Some(SetAgentDrivers {
            codex: Some(AgentDriver::Acp),
            ..SetAgentDrivers::default()
        }),
    };
    assert_eq!(drivers.into_call().since(), Some(ProtocolVersion::new(18)));
}

/// Protocol 11's write guard (the join of wave 2's first half): a write
/// guarded by its content needs 11, which a protocol 10 node could not
/// decode; one that creates only, or writes whatever is there, does not.
#[test]
fn a_write_guarded_by_its_content_needs_protocol_eleven() {
    use crate::workspace::{ContentHash, WorkspaceRoot, WriteExpect, WriteFile};
    let write = |expect| {
        WriteFile {
            root: WorkspaceRoot::Project {
                project: crate::id::ProjectId::random(),
            },
            path: "a.txt".parse().unwrap(),
            text: String::new(),
            expect,
        }
        .into_call()
    };
    assert_eq!(write(WriteExpect::Absent).since(), Some(V10));
    assert_eq!(write(WriteExpect::Any).since(), Some(V10));
    let hash = ContentHash::from_sha256([7; 32]);
    assert_eq!(
        write(WriteExpect::Content { hash }).since(),
        Some(ProtocolVersion::new(11))
    );
}

#[test]
fn a_new_field_raises_what_a_call_needs() {
    let terminal = SessionMode::Terminal {
        args: Vec::new(),
        size: TerminalSize::default(),
    };
    let plain = SessionSpec::new(HarnessKind::Codex, terminal);
    let create = |spec| CreateSession { spec }.into_call();
    assert_eq!(create(plain.clone()).since(), Some(ProtocolVersion::new(2)));
    let profiled = plain.with_profile("alt".parse().unwrap());
    assert_eq!(create(profiled).since(), Some(V5));

    let invite = |scopes| {
        InviteDevice {
            label: None,
            ttl_secs: 60,
            scopes,
        }
        .into_call()
    };
    assert_eq!(invite(None).since(), Some(V4));
    assert_eq!(invite(Some(vec![Scope::Sync])).since(), Some(V5));
    // A protocol 5 node would refuse a scope it does not know.
    assert_eq!(
        invite(Some(vec![Scope::Sync, Scope::Relay])).since(),
        Some(V6)
    );

    let read = |wait| {
        ReadEvents {
            session: crate::id::SessionId::random(),
            from: EventSeq::ZERO,
            limit: 10,
            wait,
        }
        .into_call()
    };
    assert_eq!(read(None).since(), Some(ProtocolVersion::new(2)));
    assert_eq!(read(Some(30)).since(), Some(V6));

    // Protocol 9: an older node would start the session in the agent's
    // default mode, or unnamed, rather than refuse.
    let structured = |options| SessionMode::Structured {
        model: None,
        resume: None,
        options,
    };
    let chosen = vec![crate::options::OptionChoice {
        id: crate::id::OptionId::new("mode"),
        value: crate::options::ChoiceValue::Select {
            value: crate::id::OptionValueId::new("plan"),
        },
    }];
    let plain = SessionSpec::new(HarnessKind::ClaudeCode, structured(Vec::new()));
    assert_eq!(create(plain.clone()).since(), Some(ProtocolVersion::new(2)));
    assert_eq!(
        create(plain.clone().with_title("Fix the build".parse().unwrap())).since(),
        Some(V9)
    );
    let choosing = SessionSpec::new(HarnessKind::ClaudeCode, structured(chosen))
        .with_profile("alt".parse().unwrap());
    assert_eq!(create(choosing).since(), Some(V9));
}

#[test]
fn options_and_titles_read_as_their_wire_form() {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: vec![crate::options::OptionChoice {
                id: crate::id::OptionId::new("mode"),
                value: crate::options::ChoiceValue::Select {
                    value: crate::id::OptionValueId::new("plan"),
                },
            }],
        },
    )
    .with_title("Fix the build".parse().unwrap());
    let text = serde_json::to_string(&CreateSession { spec: spec.clone() }.into_call()).unwrap();
    assert_eq!(
        text,
        r#"{"method":"session.create","params":{"spec":{"harness":"claude-code","title":"Fix the build","mode":"structured","options":[{"id":"mode","kind":"select","value":"plan"}]}}}"#
    );
    assert_eq!(
        serde_json::from_str::<Call>(&text).unwrap(),
        CreateSession { spec: spec.clone() }.into_call()
    );

    // A session that has none says nothing of either, as in protocol 8.
    let bare = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let info = SessionInfo::new(crate::id::SessionId::random(), bare, DateTime::UNIX_EPOCH);
    let text = serde_json::to_string(&info).unwrap();
    assert!(
        !text.contains("title") && !text.contains("options"),
        "{text}"
    );
    // A spec's title is the session's until something renames it.
    let named = SessionInfo::new(crate::id::SessionId::random(), spec, DateTime::UNIX_EPOCH);
    assert_eq!(
        named.title.as_ref().map(crate::title::SessionTitle::as_str),
        Some("Fix the build")
    );

    let session = crate::id::SessionId::random();
    let rename = RenameSession {
        session,
        title: "Release notes".parse().unwrap(),
    }
    .into_call();
    let text = serde_json::to_string(&rename).unwrap();
    assert_eq!(
        text,
        format!(
            r#"{{"method":"session.rename","params":{{"session":"{session}","title":"Release notes"}}}}"#
        )
    );
    assert_eq!(serde_json::from_str::<Call>(&text).unwrap(), rename);
    let set = SetSessionOption {
        session,
        choice: crate::options::OptionChoice {
            id: crate::id::OptionId::new("fast"),
            value: crate::options::ChoiceValue::Toggle { on: true },
        },
    }
    .into_call();
    let text = serde_json::to_string(&set).unwrap();
    assert_eq!(
        text,
        format!(
            r#"{{"method":"session.set_option","params":{{"session":"{session}","choice":{{"id":"fast","kind":"toggle","on":true}}}}}}"#
        )
    );
    assert_eq!(serde_json::from_str::<Call>(&text).unwrap(), set);
}

/// Protocol 9's events: a protocol 8 client gets a line in their place,
/// at the same position.
#[test]
fn options_and_renames_reach_older_clients_as_lines() {
    use crate::options::{OptionCategory, OptionKind, OptionValue, SessionOption};
    let options = vec![
        SessionOption {
            id: crate::id::OptionId::new("model"),
            name: "Model".to_owned(),
            description: None,
            category: OptionCategory::Model,
            kind: OptionKind::Select {
                current: crate::id::OptionValueId::new("haiku"),
                values: vec![OptionValue {
                    id: crate::id::OptionValueId::new("haiku"),
                    name: "Haiku".to_owned(),
                    description: None,
                    group: None,
                }],
            },
        },
        SessionOption {
            id: crate::id::OptionId::new("mode"),
            name: "Mode".to_owned(),
            description: None,
            category: OptionCategory::Mode,
            // A current value the agent does not list shows as its id.
            kind: OptionKind::Select {
                current: crate::id::OptionValueId::new("plan"),
                values: Vec::new(),
            },
        },
        SessionOption {
            id: crate::id::OptionId::new("fast"),
            name: "Fast mode".to_owned(),
            description: None,
            category: OptionCategory::Other,
            kind: OptionKind::Toggle { on: false },
        },
    ];
    let line = |line: &str| {
        Some(SessionEvent::Agent {
            event: AgentEvent::HarnessLog {
                line: line.to_owned(),
            },
        })
    };
    let changed = SessionEvent::OptionsChanged { options };
    assert_eq!(changed.required_protocol(), Some(V9));
    assert_eq!(changed.for_protocol(V9), None);
    assert_eq!(
        changed.for_protocol(V8),
        line("vessel: options: Model Haiku, Mode plan, Fast mode off")
    );
    let none = SessionEvent::OptionsChanged {
        options: Vec::new(),
    };
    assert_eq!(none.for_protocol(V8), line("vessel: options: none"));
    let text = serde_json::to_string(&none).unwrap();
    assert_eq!(text, r#"{"type":"options_changed","options":[]}"#);

    let renamed = SessionEvent::Renamed {
        title: "Fix the build".parse().unwrap(),
        origin: crate::title::TitleOrigin::Unknown,
    };
    assert_eq!(renamed.required_protocol(), Some(V9));
    assert_eq!(
        renamed.for_protocol(V4),
        line("vessel: renamed to Fix the build")
    );
    let text = serde_json::to_string(&renamed).unwrap();
    assert_eq!(text, r#"{"type":"renamed","title":"Fix the build"}"#);
    assert_eq!(
        serde_json::from_str::<SessionEvent>(&text).unwrap(),
        renamed
    );

    let frame = ServerFrame::Event {
        subscription: SubscriptionId::new(1),
        record: record(4, renamed),
    };
    let Some(ServerFrame::Event {
        record: stand_in, ..
    }) = frame.for_protocol(V8)
    else {
        panic!("an event frame stays an event frame");
    };
    assert_eq!(stand_in.seq, EventSeq::new(4));
}

fn record(seq: u64, event: SessionEvent) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event,
    }
}

fn moved() -> SessionEvent {
    SessionEvent::ProfileChanged {
        from: None,
        to: Some("alt".parse().unwrap()),
    }
}

#[test]
fn older_connections_get_stand_ins_at_the_same_position() {
    let event = ServerFrame::Event {
        subscription: SubscriptionId::new(1),
        record: record(7, moved()),
    };
    assert_eq!(event.for_protocol(V5), None);
    let Some(ServerFrame::Event {
        record: stand_in, ..
    }) = event.for_protocol(V4)
    else {
        panic!("an event frame stays an event frame");
    };
    assert_eq!(stand_in.seq, EventSeq::new(7));
    assert_eq!(
        stand_in.event,
        SessionEvent::Agent {
            event: AgentEvent::HarnessLog {
                line: "vessel: profile changed from default to alt".to_owned()
            }
        }
    );

    let status = record_status();
    let page = |records| ServerFrame::Response {
        id: RequestId::new(3),
        outcome: Outcome::Ok(Reply::ReadEvents(EventPage {
            records,
            next: EventSeq::new(9),
            at_end: true,
        })),
    };
    assert_eq!(page(vec![status.clone()]).for_protocol(V4), None);
    let Some(ServerFrame::Response {
        outcome: Outcome::Ok(Reply::ReadEvents(downgraded)),
        ..
    }) = page(vec![status.clone(), record(8, moved())]).for_protocol(V4)
    else {
        panic!("a page stays a page");
    };
    assert_eq!(downgraded.records.first(), Some(&status));
    assert_eq!(
        downgraded.records.iter().map(|r| r.seq).collect::<Vec<_>>(),
        [EventSeq::new(1), EventSeq::new(8)]
    );
    let text = serde_json::to_string(&downgraded).unwrap();
    assert!(!text.contains("profile_changed"), "{text}");
}

/// Protocol 7's model call: a protocol 6 client gets a line with the
/// model and the token counts in its place, or the failure's class; never
/// anything else of the call.
#[test]
fn a_model_call_reaches_older_clients_as_a_line() {
    let call: crate::model::ModelCall = serde_json::from_str(
        r#"{"route":"qwen-coder","model":"qwen/qwen3-coder","api":"openai_chat_completions",
            "usage":{"input":72,"cache_read":128,"output":50},"stop":"tool_use","duration_ms":900}"#,
    )
    .unwrap();
    let event = SessionEvent::ModelCall(call.clone());
    assert_eq!(event.for_protocol(ProtocolVersion::new(7)), None);
    assert_eq!(
        event.for_protocol(V6),
        Some(SessionEvent::Agent {
            event: AgentEvent::HarnessLog {
                line: "vessel: model call to qwen/qwen3-coder: 200 in, 50 out".to_owned()
            }
        })
    );
    let failed = SessionEvent::ModelCall(crate::model::ModelCall {
        stop: crate::model::ModelStop::Error,
        failure: Some(crate::model::ModelFailure::ContextTooLong),
        ..call
    });
    assert_eq!(
        failed.for_protocol(V4),
        Some(SessionEvent::Agent {
            event: AgentEvent::HarnessLog {
                line: "vessel: model call to qwen/qwen3-coder failed (context_too_long)".to_owned()
            }
        })
    );
    let text = serde_json::to_string(&failed).unwrap();
    assert!(
        text.starts_with(r#"{"type":"model_call","route":"qwen-coder""#),
        "{text}"
    );
}

fn record_status() -> EventRecord {
    record(
        1,
        SessionEvent::Status {
            status: crate::session::SessionStatus::Running,
        },
    )
}

#[test]
fn older_connections_do_not_see_newer_scopes() {
    let device = DeviceInfo {
        id: DeviceId::random(),
        label: "member".to_owned(),
        created_at: DateTime::UNIX_EPOCH,
        last_seen: None,
        scopes: vec![Scope::Read, Scope::Sync, Scope::Relay],
        revoked_at: None,
    };
    let frame = ServerFrame::Response {
        id: RequestId::new(1),
        outcome: Outcome::Ok(Reply::ListDevices(vec![device])),
    };
    assert_eq!(frame.for_protocol(V6), None);
    let scopes_at = |protocol| {
        let Some(ServerFrame::Response {
            outcome: Outcome::Ok(Reply::ListDevices(devices)),
            ..
        }) = frame.for_protocol(protocol)
        else {
            panic!("a device list stays a device list");
        };
        devices
            .iter()
            .flat_map(|d| d.scopes.clone())
            .collect::<Vec<_>>()
    };
    // A protocol 5 client cannot decode `relay`, a protocol 4 one `sync`.
    assert_eq!(scopes_at(V5), [Scope::Read, Scope::Sync]);
    assert_eq!(scopes_at(V4), [Scope::Read]);
}

#[test]
fn where_a_session_came_from_is_optional_and_tagged() {
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(crate::id::SessionId::random(), spec, DateTime::UNIX_EPOCH);
    // A session of this node only looks as it did in protocol 5.
    let local = serde_json::to_string(&info).unwrap();
    assert!(
        !local.contains("via") && !local.contains("origin"),
        "{local}"
    );
    info.via = Some(crate::id::NodeId::random());
    info.origin = Some(crate::session::SessionOrigin::Imported {
        node: crate::id::NodeId::random(),
        session: crate::id::SessionId::random(),
    });
    let imported = serde_json::to_string(&info).unwrap();
    assert!(
        imported.contains(r#""origin":{"kind":"imported","node":""#),
        "{imported}"
    );
    assert_eq!(
        serde_json::from_str::<SessionInfo>(&imported).unwrap(),
        info
    );
}

#[test]
fn self_inspection_is_the_only_authentication_only_method() {
    let unscoped: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.scope().is_none())
        .copied()
        .collect();
    assert_eq!(unscoped, [Method::GetDeviceSelf]);
    assert!(Method::GetDeviceSelf.forwardable());
}

#[test]
fn typed_envelopes_require_the_inner_methods_and_fields() {
    let query = crate::GetDeviceSelf {}.into_call();
    let since = query.since();
    let origin = crate::Origin {
        node: crate::NodeId::random(),
        by: crate::OriginBy::Owner,
        scopes: vec![Scope::Read],
    };
    for envelope in [
        Forward {
            node: crate::NodeId::random(),
            call: Box::new(query.clone()),
        }
        .into_call(),
        RelayCall {
            node: crate::NodeId::random(),
            call: Box::new(query.clone()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(query),
            origin,
        }
        .into_call(),
    ] {
        assert_eq!(envelope.since(), since);
    }
    let prompt = PromptSession {
        session: crate::SessionId::random(),
        content: vec![],
        delivery: crate::Delivery::Queue,
        notify: true,
    }
    .into_call();
    let expected = prompt.since();
    assert_eq!(
        Forward {
            node: crate::NodeId::random(),
            call: Box::new(prompt)
        }
        .into_call()
        .since(),
        expected
    );
}

#[test]
fn saved_limit_requests_and_sanitized_states_round_trip_with_exact_methods() {
    use crate::limits::*;
    for (call, json) in [
        (
            GetUsageLimits {
                scope: LimitsScope::Account {
                    account: LimitAccount {
                        harness: HarnessKind::Codex.into(),
                        profile: None,
                    },
                },
                refresh: LimitRefresh::Refresh,
            }
            .into_call(),
            r#"{"method":"usage.limits","params":{"scope":{"kind":"account","account":{"harness":"codex","profile":null}},"refresh":"refresh"}}"#,
        ),
        (
            SetUsageLookup {
                policy: ExternalLookup::On,
            }
            .into_call(),
            r#"{"method":"usage.lookup","params":{"policy":"on"}}"#,
        ),
    ] {
        assert_eq!(serde_json::to_string(&call).unwrap(), json);
        assert_eq!(serde_json::from_str::<Call>(json).unwrap(), call);
        assert_eq!(call.since(), Some(LIMITS_PROTOCOL_VERSION));
    }
    let json = r#"{"policy":"on","account":{"harness":"codex","profile":null},"source":"codex","state":{"state":"failed","reason":"rate_limited","retained":null,"retry_at":"2026-06-01T12:00:00Z"},"cost_visibility":"withheld","notice":null}"#;
    let report: UsageLimits = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_string(&report).unwrap(), json);
    assert_eq!(
        report.state,
        AccountLimitState::Failed {
            reason: LimitFailure::RateLimited,
            retained: None,
            retry_at: Some("2026-06-01T12:00:00Z".parse().unwrap())
        }
    );
}

mod envelope_versions;
