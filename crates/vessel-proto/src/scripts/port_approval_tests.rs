//! Declared ports cannot reach any approval reply before clients can show them.

use super::*;
use crate::rpc::Reply;
use crate::workspace::{ApprovedCommands, CommandSet, CommandsDigest, ServicePort};

fn listed() -> ProjectCommands {
    ProjectCommands::Listed {
        setup: Vec::new(),
        teardown: Vec::new(),
        terminals: Vec::new(),
        service_ports: None,
        scripts: vec![
            ScriptEntry::new(
                "dev".parse().unwrap(),
                ScriptKind::Service,
                "serve-approval-port".parse().unwrap(),
                None,
                Some(ServicePort::try_from(3210_u16).unwrap()),
            )
            .unwrap(),
        ],
        digest: CommandsDigest::from_sha256(std::array::from_fn(|_| 0xab)),
        approved: false,
        last_approved: None,
    }
}

/// Keep the direct and nested approval paths in one coverage fixture.
pub(super) fn approval_replies(commands: &ProjectCommands) -> [Reply; 6] {
    let outcome = ScriptRunOutcome::NeedsApproval {
        commands: commands.clone(),
    };
    [
        Reply::ProjectCommands(commands.clone()),
        Reply::ApproveCommands(commands.clone()),
        Reply::CreateWorktree(CreateWorktreeOutcome::NeedsApproval {
            commands: commands.clone(),
        }),
        Reply::ListScripts(ScriptList {
            root: PathBuf::from("/project"),
            commands: commands.clone(),
            runs: Vec::new(),
        }),
        Reply::RunScript(outcome.clone()),
        Reply::RestartScript(outcome),
    ]
}

fn assert_hidden(commands: &ProjectCommands) {
    let ProjectCommands::Listed { digest, .. } = commands else {
        panic!("listed");
    };
    for protocol in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
        for reply in approval_replies(commands) {
            let adapted = serde_json::to_string(&reply.for_protocol(protocol).unwrap()).unwrap();
            assert!(adapted.contains("invalid"), "{adapted}");
            assert!(!adapted.contains("serve-approval-port"), "{adapted}");
            assert!(!adapted.contains("3210"), "{adapted}");
            assert!(!adapted.contains(digest.as_str()), "{adapted}");
        }
    }
}

#[test]
fn declared_ports_are_hidden_on_every_approval_reply_until_assignment() {
    assert_hidden(&listed());
}

#[test]
fn a_removed_port_in_the_last_approval_is_hidden_too() {
    let ProjectCommands::Listed {
        scripts, digest, ..
    } = listed()
    else {
        panic!("listed");
    };
    let previous = ApprovedCommands {
        commands: CommandSet {
            scripts,
            ..CommandSet::default()
        },
        digest,
        approved_at: chrono::DateTime::UNIX_EPOCH,
    };
    let mut current = listed();
    if let ProjectCommands::Listed {
        scripts,
        last_approved,
        ..
    } = &mut current
    {
        scripts.clear();
        *last_approved = Some(Box::new(previous));
    }
    assert_hidden(&current);
}

#[test]
fn the_assigned_threshold_enables_ports_without_changing_the_feature_constant() {
    let commands = listed();
    assert_eq!(
        commands.for_project_fields(crate::PROTOCOL_VERSION, Some(crate::PROTOCOL_VERSION)),
        None
    );
    assert!(matches!(
        commands.for_project_fields(crate::MIN_PROTOCOL_VERSION, Some(crate::PROTOCOL_VERSION)),
        Some(ProjectCommands::Invalid { .. })
    ));
    assert_eq!(crate::workspace::PROJECT_FILE_KEYS, None);
}
