//! Sources: Paseo components/provider-catalog-search.test.ts (all four
//! cases), and crates/vessel-node/tests/e2e/catalogue.rs REGISTRY. The JSON
//! fixture is that registry's resolved wire reply, not a second index parser.
use super::*;
use vessel_proto::agents::{AddedAgent, Catalogue};

mod words;

fn catalogue() -> Catalogue {
    serde_json::from_str(include_str!("fixtures/catalogue.json")).unwrap()
}
fn entry(id: &str, name: &str) -> CatalogueEntry {
    CatalogueEntry {
        id: id.parse().unwrap(),
        name: name.to_owned(),
        version: "1.2.3".parse().unwrap(),
        description: "fixture description".to_owned(),
        website: None,
        repository: None,
        license: None,
        runs: CatalogueRun::Unavailable,
    }
}
fn entries() -> Vec<CatalogueEntry> {
    [
        ("kiro-cli", "Kiro CLI"),
        ("goose", "goose"),
        ("kimi", "Kimi Code CLI"),
        ("gemini", "Gemini CLI"),
        ("qwen-code", "Qwen Code"),
        ("codex-acp", "Codex"),
    ]
    .map(|(id, name)| entry(id, name))
    .into()
}
fn names<'a>(entries: &'a [CatalogueEntry], query: &str) -> Vec<&'a str> {
    ranked(entries, query)
        .into_iter()
        .map(|entry| entry.name.as_str())
        .collect()
}
fn recorded(source: AgentSource) -> AgentInfo {
    AgentInfo {
        agent: HarnessRef::Added("renamed-agent".parse().unwrap()),
        detail: AgentDetail::Added {
            added: AddedAgent {
                label: Some("A different display label".parse().unwrap()),
                program: "npx".parse().unwrap(),
                args: vec!["-y".into(), "@example/acp@1.2.3".into(), "--acp".into()],
                env: vec![],
                secrets: vec![],
                inherit: vec![],
                mcp: vec![],
                source,
            },
        },
    }
}

#[test]
fn node_fixture_keeps_the_pinned_command_and_does_not_guess_an_install() {
    let fixture = catalogue();
    assert_eq!(fixture.skipped, 1);
    assert_eq!(fixture.entries.len(), 1);
    let entry = &fixture.entries[0];
    assert_eq!(entry.id.as_str(), "example-agent");
    assert_eq!(entry.version.as_str(), "1.2.3");
    let CatalogueRun::Command {
        program, args, env, ..
    } = &entry.runs
    else {
        panic!("node fixture must retain its command")
    };
    assert_eq!(program.as_str(), "npx");
    assert_eq!(args, &["-y", "@example/acp@1.2.3", "--acp"]);
    assert_eq!(env[0].name.as_str(), "EXAMPLE_MODE");
    assert_eq!(env[0].value, "acp");
    assert_eq!(offer(entry, &[]), Offer::Add(Runner::Found));
}

#[test]
fn every_reference_catalog_search_case_uses_the_same_titles_and_results() {
    let entries = entries();
    for (query, expected) in [
        (
            "  ",
            vec![
                "Codex",
                "Gemini CLI",
                "goose",
                "Kimi Code CLI",
                "Kiro CLI",
                "Qwen Code",
            ],
        ),
        ("cod", vec!["Codex", "Kimi Code CLI", "Qwen Code"]),
        ("gmni", vec!["Gemini CLI"]),
        ("qw cod", vec!["Qwen Code"]),
        ("kiro-cli", vec!["Kiro CLI"]),
        ("absent", vec![]),
    ] {
        assert_eq!(names(&entries, query), expected, "query {query:?}");
    }
    assert_eq!(names(&[], ""), Vec::<&str>::new());
    assert_eq!(
        entries[0].name, "Kiro CLI",
        "borrowed input was not reordered"
    );
}

#[test]
fn tied_scores_use_shared_case_order_and_descriptions_are_not_search_fields() {
    let mut entries = [
        entry("one", "alpha"),
        entry("two", "Alpha"),
        entry("three", "ALPHA"),
    ];
    entries[0].description = "secret search phrase".into();
    assert_eq!(names(&entries, "alpha"), ["ALPHA", "Alpha", "alpha"]);
    assert!(names(&entries, "secret search").is_empty());
}

#[test]
fn only_command_distributions_offer_add_even_when_the_runner_is_missing() {
    let mut fixture = catalogue();
    let entry = &mut fixture.entries[0];
    let CatalogueRun::Command { runner_found, .. } = &mut entry.runs else {
        panic!("command fixture")
    };
    *runner_found = false;
    assert_eq!(offer(entry, &[]), Offer::Add(Runner::Missing));
    for harness in HarnessKind::ALL {
        entry.runs = CatalogueRun::Known { harness };
        assert_eq!(offer(entry, &[]), Offer::BuiltIn(harness));
    }
    let archive: HttpUrl = "https://example.invalid/agent.zip".parse().unwrap();
    entry.runs = CatalogueRun::Download {
        archive: archive.clone(),
        sha256: None,
    };
    assert_eq!(offer(entry, &[]), Offer::Download(&archive));
    entry.runs = CatalogueRun::Unavailable;
    assert_eq!(offer(entry, &[]), Offer::Unavailable);
}

#[test]
fn recorded_status_uses_registry_id_not_name_version_or_program_similarity() {
    let fixture = catalogue();
    let entry = &fixture.entries[0];
    let agents = [recorded(AgentSource::Registry {
        id: entry.id.clone(),
        version: "0.9.0".parse().unwrap(),
    })];
    assert_eq!(offer(entry, &agents), Offer::Recorded(&agents[0].agent));
    let manual = [recorded(AgentSource::Command)];
    assert_eq!(offer(entry, &manual), Offer::Add(Runner::Found));
    let other = [recorded(AgentSource::Registry {
        id: "another-entry".parse().unwrap(),
        version: entry.version.clone(),
    })];
    assert_eq!(offer(entry, &other), Offer::Add(Runner::Found));
    let known = [AgentInfo {
        agent: HarnessRef::Known(HarnessKind::Codex),
        detail: AgentDetail::Known {
            installs: vec![],
            sign_in: vessel_proto::agents::SignInHelp::default(),
        },
    }];
    assert_eq!(offer(entry, &known), Offer::Add(Runner::Found));
}

mod addition_words;
mod resources;
