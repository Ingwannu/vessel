//! A block is approval data even when it names no service to start.

use super::*;
use crate::workspace::{ApprovedCommands, CommandSet, CommandsDigest, ServicePorts};

fn allocation() -> ServicePorts {
    ServicePorts::new(
        Some("7000-7009".parse().unwrap()),
        Some("tools/port-approval".parse().unwrap()),
    )
    .unwrap()
}

fn listed() -> ProjectCommands {
    ProjectCommands::Listed {
        setup: Vec::new(),
        teardown: Vec::new(),
        scripts: Vec::new(),
        terminals: Vec::new(),
        service_ports: Some(allocation()),
        digest: CommandsDigest::from_sha256(std::array::from_fn(|_| 0xcd)),
        approved: false,
        last_approved: None,
    }
}

fn assert_hidden(commands: &ProjectCommands) {
    let ProjectCommands::Listed { digest, .. } = commands else {
        panic!("listed");
    };
    for protocol in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
        for reply in port_approval_tests::approval_replies(commands) {
            let encoded = serde_json::to_string(&reply.for_protocol(protocol).unwrap()).unwrap();
            assert!(encoded.contains("invalid"), "{encoded}");
            assert!(!encoded.contains("7000-7009"), "{encoded}");
            assert!(!encoded.contains("tools/port-approval"), "{encoded}");
            assert!(!encoded.contains(digest.as_str()), "{encoded}");
        }
    }
}

#[test]
fn allocation_only_sets_are_hidden_on_every_approval_reply_until_assignment() {
    assert_hidden(&listed());
}

#[test]
fn a_removed_allocation_in_the_prior_approval_is_hidden_too() {
    let mut current = listed();
    if let ProjectCommands::Listed {
        service_ports,
        digest,
        last_approved,
        ..
    } = &mut current
    {
        let prior = ApprovedCommands {
            commands: CommandSet {
                service_ports: service_ports.take(),
                ..CommandSet::default()
            },
            digest: digest.clone(),
            approved_at: chrono::DateTime::UNIX_EPOCH,
        };
        *last_approved = Some(Box::new(prior));
    }
    assert_hidden(&current);
}

#[test]
fn a_synthetic_assignment_enables_the_complete_block_without_assigning_a_number() {
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

#[test]
fn older_nodes_omitting_the_block_keep_their_existing_wire_shape() {
    let old = r#"{"setup":[],"teardown":[]}"#;
    let decoded: CommandSet = serde_json::from_str(old).unwrap();
    assert_eq!(decoded.service_ports, None);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), old);
}
