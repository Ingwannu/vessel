use std::fmt::Write as _;

use super::*;
use crate::{DurationMs, ErrorCode, HarnessKind, NodeId, NodeState, agents::UnknownReason};
use chrono::{DateTime, Utc};

const PRIVATE_WORDS: &str = "PRIVATE-CANARY-NOT-FOR-REPORTS";
fn at() -> DateTime<Utc> {
    DateTime::UNIX_EPOCH
}
fn unavailable<T>() -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable {
        reason: DiagnosticUnavailable::NotChecked,
    }
}
fn available<T>(value: T) -> DiagnosticSection<T> {
    DiagnosticSection::Available { at: at(), value }
}
fn fields() -> DiagnosticReportFields {
    DiagnosticReportFields {
        collected_at: at(),
        node: DiagnosticNode {
            id: NodeId::random(),
            version: Some("0.0.0".parse().unwrap()),
            os: DiagnosticOs::Linux,
            arch: DiagnosticArch::X86_64,
            state: NodeState::Serving,
            uptime: DurationMs::from_duration(std::time::Duration::ZERO),
            home: DiagnosticHome::Redacted,
        },
        process: unavailable(),
        store: unavailable(),
        agents: unavailable(),
        sessions: unavailable(),
        listeners: unavailable(),
        relays: unavailable(),
        recent_failures: unavailable(),
    }
}
fn agent(slot: usize) -> DiagnosticAgent {
    DiagnosticAgent {
        id: DiagnosticAgentId::Custom {
            slot: slot.try_into().unwrap(),
        },
        executable: DiagnosticPresence::Found,
        structured: DiagnosticAvailability::Unknown {
            reason: UnknownReason::NotChecked,
        },
        terminal: DiagnosticAvailability::Unavailable {
            reason: DiagnosticAgentRefusal::Unsupported,
        },
        sign_in: DiagnosticSignIn::Unknown {
            reason: UnknownReason::NotChecked,
        },
        version: None,
        checked_at: None,
        stale: false,
    }
}
fn agents() -> Vec<DiagnosticAgent> {
    (MIN_CUSTOM_AGENT_SLOT..=MAX_DIAGNOSTIC_AGENTS)
        .map(agent)
        .collect()
}
fn failure() -> DiagnosticFailure {
    DiagnosticFailure {
        code: ErrorCode::Internal,
        at: at(),
    }
}

#[test]
fn feature_is_assigned_and_required_by_protocol() {
    use crate::Request as _;
    let method = serde_json::from_str::<crate::Method>(r#""node.diagnostics""#).unwrap();
    assert_eq!(method, crate::Method::Diagnostics);
    assert!(DIAGNOSTICS_PROTOCOL.is_some());
    assert_eq!(method.since(), DIAGNOSTICS_PROTOCOL);
    assert_eq!(
        GetDiagnostics {}.into_call().required_protocol(),
        DIAGNOSTICS_PROTOCOL
    );
}

#[test]
fn report_round_trip_has_only_safe_value_fields() {
    let mut source = fields();
    source.node.version = Some(format!("0.0.0-{PRIVATE_WORDS}").parse().unwrap());
    source.agents = available(
        vec![DiagnosticAgent {
            id: DiagnosticAgentId::Builtin {
                harness: HarnessKind::Codex,
            },
            ..agent(MIN_CUSTOM_AGENT_SLOT)
        }]
        .try_into()
        .unwrap(),
    );
    let report = DiagnosticReport::try_from(source).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains(PRIVATE_WORDS));
    assert!(!format!("{report:?}").contains(PRIVATE_WORDS));
    assert_eq!(
        serde_json::from_str::<DiagnosticReport>(&json).unwrap(),
        report
    );
}

#[test]
fn version_bounds_and_suffixes_never_echo_input() {
    let prefix = "0.0.0-";
    let exact = format!(
        "{prefix}{}",
        "x".repeat(MAX_DIAGNOSTIC_VERSION_BYTES.saturating_sub(prefix.len()))
    );
    let normalized: DiagnosticVersion = exact.parse().unwrap();
    assert!(normalized.prerelease);
    let excess = format!("{exact}x");
    let error = excess.parse::<DiagnosticVersion>().unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_VERSION_BYTES.to_string())
    );
    assert!(!error.to_string().contains(&excess));
    assert!(
        format!("{MAX_DIAGNOSTIC_VERSION_COMPONENT}.0.0")
            .parse::<DiagnosticVersion>()
            .is_ok()
    );
    let overflow = u64::from(MAX_DIAGNOSTIC_VERSION_COMPONENT).saturating_add(1);
    assert!(
        format!("{overflow}.0.0")
            .parse::<DiagnosticVersion>()
            .is_err()
    );
    for bad in [
        "00.0.0",
        "0.0",
        "0.0.0.0",
        "0.0.0-",
        "0.0.0-00",
        "0.0.0+",
        PRIVATE_WORDS,
    ] {
        assert!(bad.parse::<DiagnosticVersion>().is_err());
    }
}

#[test]
fn count_saturation_is_exact_for_json_clients() {
    let full = DiagnosticCount::new(MAX_DIAGNOSTIC_COUNT);
    let overflow = full.saturating_add(DiagnosticCount::new(1));
    assert_eq!(overflow.get(), MAX_DIAGNOSTIC_COUNT);
    assert!(overflow.is_saturated());
    assert!(DiagnosticCount::new(MAX_DIAGNOSTIC_COUNT.saturating_add(1)).is_saturated());
    let invalid = format!(
        r#"{{"value":{},"saturated":false}}"#,
        MAX_DIAGNOSTIC_COUNT.saturating_add(1)
    );
    let error = serde_json::from_str::<DiagnosticCount>(&invalid).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_COUNT.to_string())
    );
}

#[test]
fn slot_and_schema_bounds_are_named_and_redacted() {
    assert!(CustomAgentSlot::try_from(MIN_CUSTOM_AGENT_SLOT.saturating_sub(1)).is_err());
    assert!(CustomAgentSlot::try_from(MAX_DIAGNOSTIC_AGENTS).is_ok());
    let error = CustomAgentSlot::try_from(MAX_DIAGNOSTIC_AGENTS.saturating_add(1)).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_AGENTS.to_string())
    );
    assert!(DiagnosticSchemaVersion::try_from(MAX_DIAGNOSTIC_SCHEMA).is_ok());
    assert!(DiagnosticSchemaVersion::try_from(MAX_DIAGNOSTIC_SCHEMA.saturating_add(1)).is_err());
    let invalid = format!("\"{PRIVATE_WORDS}\"");
    let error = serde_json::from_str::<CustomAgentSlot>(&invalid).unwrap_err();
    assert!(!format!("{error:?}").contains(PRIVATE_WORDS));
}

#[test]
fn bounded_lists_refuse_first_excess_before_its_body() {
    let rows = agents();
    assert_eq!(
        DiagnosticAgents::try_from(rows.clone()).unwrap().len(),
        MAX_DIAGNOSTIC_AGENTS
    );
    let mut excess = rows.clone();
    excess.push(agent(MIN_CUSTOM_AGENT_SLOT));
    assert!(DiagnosticAgents::try_from(excess).is_err());
    let mut encoded = serde_json::to_string(&rows).unwrap();
    assert_eq!(encoded.pop(), Some(']'));
    write!(encoded, ",{PRIVATE_WORDS}]").unwrap();
    let error = serde_json::from_str::<DiagnosticAgents>(&encoded).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_AGENTS.to_string())
    );
    assert!(!error.to_string().contains(PRIVATE_WORDS));
    let retained = vec![failure(); MAX_DIAGNOSTIC_FAILURES];
    assert!(RecentFailures::try_from(retained.clone()).is_ok());
    let mut excess = retained;
    excess.push(failure());
    let error = serde_json::from_str::<RecentFailures>(&serde_json::to_string(&excess).unwrap())
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_FAILURES.to_string())
    );
    assert!(
        DiagnosticAgents::try_from(vec![
            agent(MIN_CUSTOM_AGENT_SLOT),
            agent(MIN_CUSTOM_AGENT_SLOT)
        ])
        .is_err()
    );
}

#[test]
fn full_root_redacts_unknown_names_and_variants() {
    let report = DiagnosticReport::try_from(fields()).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    let wrong_variant = json.replace("linux", PRIVATE_WORDS);
    let error = serde_json::from_str::<DiagnosticReport>(&wrong_variant).unwrap_err();
    assert!(!format!("{error:?}").contains(PRIVATE_WORDS));
    let mut wrong_field = json;
    assert_eq!(wrong_field.pop(), Some('}'));
    write!(wrong_field, ",\"{PRIVATE_WORDS}\":true}}").unwrap();
    let error = serde_json::from_str::<DiagnosticReport>(&wrong_field).unwrap_err();
    assert!(!format!("{error:?}").contains(PRIVATE_WORDS));
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_REPORT_BYTES.to_string())
    );
}

#[test]
fn largest_collections_fit_the_root_budget_and_mutation_is_not_exposed() {
    let mut source = fields();
    source.agents = available(agents().try_into().unwrap());
    source.recent_failures = available(DiagnosticFailureWindow {
        coverage: DiagnosticErrorCoverage::LocalRpcCompletions,
        observed_since: at(),
        entries: vec![failure(); MAX_DIAGNOSTIC_FAILURES].try_into().unwrap(),
        omitted: DiagnosticCount::new(MAX_DIAGNOSTIC_COUNT),
    });
    let report = DiagnosticReport::try_from(source).unwrap();
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= MAX_DIAGNOSTIC_REPORT_BYTES);
    assert_eq!(
        serde_json::from_slice::<DiagnosticReport>(&bytes).unwrap(),
        report
    );
}

#[test]
fn contradictory_session_counts_are_not_a_valid_report() {
    let mut source = fields();
    source.sessions = available(DiagnosticRuntimeSessions {
        live: DiagnosticCount::default(),
        parked: DiagnosticCount::new(1),
    });
    assert_eq!(
        DiagnosticReport::try_from(source).unwrap_err(),
        InvalidDiagnostic::CountsDisagree
    );
}

#[test]
fn agent_protocol_has_its_own_bounded_domain() {
    let limit = u32::from(MAX_DIAGNOSTIC_AGENT_PROTOCOL);
    let version = AcpProtocolVersion::try_from(limit).unwrap();
    assert_eq!(version.get(), limit);
    assert_eq!(
        serde_json::from_str::<AcpProtocolVersion>(&serde_json::to_string(&version).unwrap())
            .unwrap(),
        version
    );
    let error = AcpProtocolVersion::try_from(limit.saturating_add(1)).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MAX_DIAGNOSTIC_AGENT_PROTOCOL.to_string())
    );
    let bad = format!("\"{PRIVATE_WORDS}\"");
    let error = serde_json::from_str::<AcpProtocolVersion>(&bad).unwrap_err();
    assert!(!format!("{error:?}").contains(PRIVATE_WORDS));
}

#[test]
fn runtime_count_certainty_cannot_contradict_its_subset() {
    let exact = DiagnosticCount::new(MAX_DIAGNOSTIC_COUNT);
    let more = DiagnosticCount::new(MAX_DIAGNOSTIC_COUNT.saturating_add(1));
    let mut source = fields();
    source.sessions = available(DiagnosticRuntimeSessions {
        live: exact,
        parked: more,
    });
    assert_eq!(
        DiagnosticReport::try_from(source.clone()).unwrap_err(),
        InvalidDiagnostic::CountsDisagree
    );
    // An exact subset or two lower bounds do not assert a contradiction.
    for parked in [exact, more] {
        source.sessions = available(DiagnosticRuntimeSessions { live: more, parked });
        assert!(DiagnosticReport::try_from(source.clone()).is_ok());
    }
}

#[test]
fn process_identity_round_trips_and_root_redacts_invalid_pid_input() {
    let mut source = fields();
    source.process = available(DiagnosticProcess {
        pid: MIN_DIAGNOSTIC_PROCESS_ID.try_into().unwrap(),
        parent: None,
        memory: unavailable(),
    });
    let report = DiagnosticReport::try_from(source).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(
        serde_json::from_str::<DiagnosticReport>(&json).unwrap(),
        report
    );
    let key = format!("\"pid\":{MIN_DIAGNOSTIC_PROCESS_ID}");
    assert!(json.contains(&key));
    for invalid in [
        MIN_DIAGNOSTIC_PROCESS_ID
            .checked_sub(1)
            .unwrap()
            .to_string(),
        u64::from(u32::MAX).checked_add(1).unwrap().to_string(),
        format!("\"{PRIVATE_WORDS}\""),
    ] {
        let input = json.replacen(&key, &format!("\"pid\":{invalid}"), 1);
        let error = serde_json::from_str::<DiagnosticReport>(&input).unwrap_err();
        assert!(!error.to_string().contains(PRIVATE_WORDS));
        assert!(!format!("{error:?}").contains(PRIVATE_WORDS));
    }
}
