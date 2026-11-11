//! Protocol 9 over the wire (milestone 10, ADR 0022): what an agent offers
//! before a session, what a session runs with and changing it, and a
//! title that survives the node's restart.

use vessel_proto::{
    ChoiceValue, ErrorCode, EventSeq, HarnessKind, OptionChoice, OptionId, OptionValueId,
    SessionEvent, SessionOption, SessionStatus, SessionTitle,
};
use vessel_rpc::CallError;

use crate::support::{Fixture, read_until, status_of, structured_spec, within};

fn code<T: std::fmt::Debug>(result: Result<T, CallError>) -> ErrorCode {
    match result {
        Err(CallError::Remote(error)) => error.code,
        other => panic!("expected a refusal from the node, got {other:?}"),
    }
}

fn select(id: &str, value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}

fn mode(options: &[SessionOption]) -> Option<&str> {
    options
        .iter()
        .find(|option| option.id.as_str() == "mode")
        .and_then(SessionOption::current_name)
}

/// `harness.options`: the agent's own options, the same answer to a second
/// caller (kept, not probed again), an empty list for a harness that
/// cannot run structured sessions here, typed errors otherwise.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_harness_offers_what_its_agent_reports() {
    within(async {
        let fixture = Fixture::with_codex_terminal_only();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;

        let offered = client
            .harness_options(HarnessKind::ClaudeCode, None)
            .await
            .unwrap();
        let ids: Vec<_> = offered
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        assert_eq!(ids, ["mode", "model", "effort", "fast"]);
        let again = client
            .harness_options(HarnessKind::ClaudeCode, None)
            .await
            .unwrap();
        assert_eq!(again, offered, "kept: the same probe's answer");

        let codex = client
            .harness_options(HarnessKind::Codex, None)
            .await
            .unwrap();
        assert!(codex.options.is_empty(), "no structured sessions here");
        assert_eq!(
            code(client.harness_options(HarnessKind::Gemini, None).await),
            ErrorCode::HarnessNotInstalled
        );
        assert_eq!(
            code(
                client
                    .harness_options(HarnessKind::ClaudeCode, Some("nobody".parse().unwrap()))
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A session's options over the wire, a change and its record; a title
/// given by `session.rename` is in the log and the info, and read back
/// from the store after a restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn options_change_and_a_rename_survives_a_restart() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let id = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        let first = seen.iter().find_map(|record| {
            if let SessionEvent::OptionsChanged { options } = &record.event {
                Some(options.clone())
            } else {
                None
            }
        });
        assert_eq!(
            first.as_deref().and_then(mode),
            Some("Default"),
            "logged before it ran"
        );
        assert_eq!(mode(&client.session_options(id).await.unwrap()), Some("Default"));

        let changed = client.set_option(id, select("mode", "plan")).await.unwrap();
        assert_eq!(mode(&changed), Some("Plan"));
        read_until(&mut stream, &mut seen, |record| {
            matches!(&record.event, SessionEvent::OptionsChanged { options } if mode(options) == Some("Plan"))
        })
        .await;
        assert_eq!(
            code(client.set_option(id, select("mode", "locked")).await),
            ErrorCode::Conflict,
            "the agent's refusal"
        );

        let title: SessionTitle = "Release notes".parse().unwrap();
        let renamed = client.rename(id, title.clone()).await.unwrap();
        assert_eq!(renamed.title.as_ref(), Some(&title));
        read_until(&mut stream, &mut seen, |record| {
            matches!(&record.event, SessionEvent::Renamed { title: logged } if *logged == title)
        })
        .await;

        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let restored = client.session(id).await.unwrap();
        assert_eq!(restored.title, Some(title), "the rename, from the store");
        assert_eq!(
            code(client.session_options(id).await),
            ErrorCode::Conflict,
            "an ended session has no agent to ask"
        );
        assert_eq!(
            code(client.rename(id, "Too late".parse().unwrap()).await),
            ErrorCode::Conflict
        );
        node.stop().await.unwrap();
    })
    .await;
}
