//! Older peers and newer agents.

use chrono::DateTime;

use super::*;
use crate::agents::{HarnessRef, TerminalSupport};
use crate::harness::{Harness, HarnessKind, StructuredSupport};
use crate::id::SessionId;
use crate::profile::ProfileInfo;
use crate::rpc::{Outcome, ServerFrame};
use crate::session::{SessionMode, SessionSpec};

const V10: ProtocolVersion = ProtocolVersion::new(10);
const V11: ProtocolVersion = ProtocolVersion::new(11);
/// The newest peer that does not read the seven.
const JUST_OLDER: ProtocolVersion = ProtocolVersion::new(READS_TEN_AGENTS.get().saturating_sub(1));

fn session(harness: impl Into<HarnessRef>) -> SessionInfo {
    let mode = SessionMode::Structured {
        model: None,
        resume: None,
        options: Vec::new(),
    };
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(harness, mode),
        DateTime::UNIX_EPOCH,
    )
}

fn goose() -> HarnessRef {
    HarnessRef::Added("goose".parse().unwrap())
}

#[test]
fn lists_leave_added_agents_out_for_an_older_peer() {
    let known = session(HarnessKind::Codex);
    let added = session(goose());
    let listed = Reply::ListSessions(vec![known.clone(), added]);
    assert_eq!(listed.for_protocol(V11), None);
    assert_eq!(
        listed.for_protocol(V10),
        Some(Reply::ListSessions(vec![known]))
    );

    let harness = |kind: HarnessRef| Harness {
        kind,
        path: "/bin/agent".into(),
        version: None,
        structured: StructuredSupport::Available,
        terminal: TerminalSupport::Available,
    };
    let found = Reply::ListHarnesses(vec![harness(HarnessKind::Gemini.into()), harness(goose())]);
    assert_eq!(found.for_protocol(V11), None);
    assert_eq!(
        found.for_protocol(V10),
        Some(Reply::ListHarnesses(vec![harness(
            HarnessKind::Gemini.into()
        )]))
    );

    let profile = |harness: HarnessRef, name: &str| ProfileInfo {
        name: name.parse().unwrap(),
        label: None,
        harness,
        sets: Vec::new(),
        model: None,
    };
    let profiles = vec![
        profile(HarnessKind::Codex.into(), "codex"),
        profile(goose(), "goose"),
    ];
    assert_eq!(
        Reply::ReloadProfiles(profiles.clone()).for_protocol(V10),
        Some(Reply::ReloadProfiles(vec![profile(
            HarnessKind::Codex.into(),
            "codex"
        )]))
    );
    assert_eq!(Reply::ListProfiles(profiles).for_protocol(V11), None);
}

#[test]
fn a_call_naming_an_added_agent_needs_protocol_11() {
    use crate::options::GetHarnessOptions;
    use crate::profile::ListProfiles;
    use crate::rpc::{Call, CreateSession};

    let mode = || SessionMode::Structured {
        model: None,
        resume: None,
        options: Vec::new(),
    };
    let added = Call::CreateSession(CreateSession {
        spec: SessionSpec::new(goose(), mode()),
    });
    assert_eq!(added.since(), V11);
    let known = Call::CreateSession(CreateSession {
        spec: SessionSpec::new(HarnessKind::Codex, mode()),
    });
    assert_eq!(known.since(), ProtocolVersion::new(2));
    let options = |harness| {
        Call::HarnessOptions(GetHarnessOptions {
            harness,
            profile: None,
        })
    };
    assert_eq!(options(goose()).since(), V11);
    assert_eq!(
        options(HarnessKind::Gemini.into()).since(),
        ProtocolVersion::new(9)
    );
    let profiles = |harness| Call::ListProfiles(ListProfiles { harness });
    assert_eq!(profiles(Some(goose())).since(), V11);
    assert_eq!(profiles(None).since(), ProtocolVersion::new(5));
}

#[test]
fn one_added_agents_session_is_withheld_from_an_older_peer() {
    let added = Reply::GetSession(session(goose()));
    assert_eq!(added.withheld(V11), None);
    assert_eq!(added.withheld(V10), Some(unreadable(&goose())));
    assert_eq!(
        Reply::GetSession(session(HarnessKind::Codex)).withheld(V10),
        None
    );
    let forwarded = Reply::Forward(Forwarded {
        reply: Box::new(added),
    });
    assert_eq!(forwarded.withheld(V10), Some(unreadable(&goose())));

    // As the node writes it: the reply becomes the error.
    let frame = ServerFrame::Response {
        id: crate::id::RequestId::new(7),
        outcome: Outcome::Ok(forwarded),
    };
    assert_eq!(frame.for_protocol(V11), None);
    assert_eq!(
        frame.for_protocol(V10),
        Some(ServerFrame::Response {
            id: crate::id::RequestId::new(7),
            outcome: Outcome::Err(unreadable(&goose())),
        })
    );
}

fn structured() -> SessionMode {
    SessionMode::Structured {
        model: None,
        resume: None,
        options: Vec::new(),
    }
}

#[test]
fn a_peer_just_older_than_the_seven_is_handed_no_grok() {
    let codex = session(HarnessKind::Codex);
    let grok = session(HarnessKind::Grok);
    let listed = Reply::ListSessions(vec![codex.clone(), grok.clone()]);
    assert_eq!(listed.for_protocol(READS_TEN_AGENTS), None);
    assert_eq!(
        listed.for_protocol(JUST_OLDER),
        Some(Reply::ListSessions(vec![codex]))
    );
    let one = Reply::GetSession(grok.clone());
    assert_eq!(one.withheld(READS_TEN_AGENTS), None);
    let refused = one.withheld(JUST_OLDER).unwrap();
    assert_eq!(refused.code, ErrorCode::Unsupported);
    assert!(refused.message.contains("grok"), "{}", refused.message);
    assert!(
        refused.message.contains(&READS_TEN_AGENTS.to_string()),
        "{}",
        refused.message
    );
    // An added agent's session is still a protocol 11 matter.
    assert_eq!(
        Reply::GetSession(session(goose())).withheld(JUST_OLDER),
        None
    );
    assert!(readable(&grok, READS_TEN_AGENTS));
    assert!(!readable(&grok, JUST_OLDER));
}

#[test]
fn agent_lists_leave_the_seven_out_for_an_older_peer_of_11_or_later() {
    use crate::agents::{AgentDetail, AgentInfo, AgentReadiness, Installed, SignedIn};

    let info = |agent: HarnessRef| AgentInfo {
        agent,
        detail: AgentDetail::Known {
            installs: Vec::new(),
            sign_in: crate::agents::SignInHelp {
                command: None,
                home: None,
                methods: Vec::new(),
            },
        },
    };
    let agents = vec![
        info(HarnessKind::Codex.into()),
        info(HarnessKind::Pi.into()),
    ];
    assert_eq!(
        Reply::ListAgents(agents.clone()).for_protocol(JUST_OLDER),
        Some(Reply::ListAgents(vec![info(HarnessKind::Codex.into())]))
    );
    assert_eq!(
        Reply::ListAgents(agents).for_protocol(READS_TEN_AGENTS),
        None
    );

    let answer = |agent: HarnessRef| AgentReadiness {
        agent,
        profile: "default".parse().unwrap(),
        installed: Installed::Missing {
            program: "agent".parse().unwrap(),
        },
        signed_in: SignedIn::Unknown {
            reason: crate::agents::UnknownReason::NotInstalled,
        },
        checked_at: DateTime::UNIX_EPOCH,
    };
    let answers = vec![answer(goose()), answer(HarnessKind::Cursor.into())];
    assert_eq!(
        Reply::GetAgentStatus(answers).for_protocol(JUST_OLDER),
        Some(Reply::GetAgentStatus(vec![answer(goose())]))
    );
}

#[test]
fn a_catalogue_entry_running_one_of_the_seven_counts_as_skipped_for_an_older_peer() {
    use crate::agents::{Catalogue, CatalogueEntry, CatalogueRun};

    let entry = |id: &str, runs: CatalogueRun| CatalogueEntry {
        id: id.parse().unwrap(),
        name: id.to_owned(),
        version: "1.0.0".parse().unwrap(),
        description: String::new(),
        website: None,
        repository: None,
        license: None,
        runs,
    };
    let codex = entry(
        "codex-acp",
        CatalogueRun::Known {
            harness: HarnessKind::Codex,
        },
    );
    let grok = entry(
        "grok-build",
        CatalogueRun::Known {
            harness: HarnessKind::Grok,
        },
    );
    let catalogue = Catalogue {
        fetched_at: None,
        version: None,
        entries: vec![codex.clone(), grok],
        skipped: 2,
    };
    let older = Reply::GetCatalogue(catalogue.clone()).for_protocol(JUST_OLDER);
    assert_eq!(
        older,
        Some(Reply::GetCatalogue(Catalogue {
            entries: vec![codex],
            skipped: 3,
            ..catalogue.clone()
        }))
    );
    assert_eq!(
        Reply::GetCatalogue(catalogue).for_protocol(READS_TEN_AGENTS),
        None
    );
}

#[test]
fn a_call_naming_one_of_the_seven_needs_their_protocol() {
    use crate::agents::{GetAgentStatus, InstallAgent};
    use crate::rpc::{Call, CreateSession};

    let create = |harness: HarnessKind| {
        Call::CreateSession(CreateSession {
            spec: SessionSpec::new(harness, structured()),
        })
    };
    assert_eq!(create(HarnessKind::Amp).since(), READS_TEN_AGENTS);
    assert_eq!(create(HarnessKind::Codex).since(), ProtocolVersion::new(2));
    let status = |agent: Option<HarnessRef>| {
        Call::GetAgentStatus(GetAgentStatus {
            agent,
            profile: None,
            refresh: false,
        })
    };
    assert_eq!(
        status(Some(HarnessKind::Muse.into())).since(),
        READS_TEN_AGENTS
    );
    assert_eq!(status(None).since(), V11);
    let install = |agent| {
        Call::InstallAgent(InstallAgent {
            agent,
            part: crate::agents::InstallPart::Harness,
        })
    };
    assert_eq!(install(HarnessKind::Omp).since(), READS_TEN_AGENTS);
    assert_eq!(install(HarnessKind::Gemini).since(), V11);
}

#[test]
fn a_usage_report_leaves_the_seven_out_for_an_older_peer_and_a_query_naming_one_needs_them() {
    use crate::rpc::Call;
    use crate::usage::{
        AgentUsage, CostVisibility, QueryUsage, UsagePeriod, UsageReport, UsageScope, UsageTotals,
    };

    let group = |harness: HarnessRef| AgentUsage {
        harness,
        profile: None,
        totals: UsageTotals::default(),
    };
    let report = UsageReport {
        at: DateTime::UNIX_EPOCH,
        accounting_started: DateTime::UNIX_EPOCH,
        retained_from: DateTime::UNIX_EPOCH,
        incomplete: false,
        cost_visibility: CostVisibility::Permitted,
        current_costs: Vec::new(),
        totals: UsageTotals::default(),
        agents: vec![
            group(HarnessKind::Codex.into()),
            group(HarnessKind::Grok.into()),
        ],
        samples: Vec::new(),
        next: None,
    };
    let older = Reply::QueryUsage(report.clone()).for_protocol(JUST_OLDER);
    assert_eq!(
        older,
        Some(Reply::QueryUsage(UsageReport {
            agents: vec![group(HarnessKind::Codex.into())],
            ..report.clone()
        }))
    );
    assert_eq!(
        Reply::QueryUsage(report).for_protocol(READS_TEN_AGENTS),
        None
    );

    let query = |scope| {
        Call::QueryUsage(QueryUsage {
            scope,
            period: UsagePeriod::Day,
            after: None,
        })
    };
    let grok = query(UsageScope::Agent {
        harness: HarnessKind::Grok.into(),
        profile: None,
    });
    assert_eq!(grok.since(), READS_TEN_AGENTS);
    let machine = query(UsageScope::Machine);
    assert_eq!(machine.since(), machine.method().since());
}
