//! Scoped real-file writer + framed actor; no process-wide config or live node.
use super::*;
use crate::dispatch::{SaveOutcome, ordered_preferences};
use crate::testing::{Home, peer, phase, state, within};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc::error::TryRecvError;
use vessel_app_view::connection::{ConnectionContext, Refusal};
use vessel_app_view::schedules::editor::{definition::Save, io};
use vessel_proto::{
    ChoiceValue, HarnessKind, OptionChoice, OptionId, OptionValueId, SessionMode, SessionSpec,
    automation::{AfterRun, CreateSchedule, ScheduleTarget},
};
fn choice() -> OptionChoice {
    OptionChoice {
        id: OptionId::new("model"),
        value: ChoiceValue::Select {
            value: OptionValueId::new("remembered-model"),
        },
    }
}
fn target() -> ScheduleTarget {
    ScheduleTarget::NewSession {
        spec: Box::new(SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![choice()],
            },
        )),
        after: AfterRun::Archive,
    }
}
fn request(
    context: ConnectionContext,
    tickets: &mut Tickets,
    target: ScheduleTarget,
) -> io::Request {
    let mut info: vessel_proto::automation::ScheduleInfo =
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../view/tests/fixtures/schedules/info.json"
        )))
        .unwrap();
    info.definition.target = target;
    io::Request::on(
        context,
        tickets.issue().unwrap(),
        io::Operation::Save(Box::new(Save::Create(CreateSchedule {
            definition: info.definition,
            paused: false,
        }))),
    )
}
fn prefs(path: &std::path::Path) -> crate::Prefs {
    let mut prefs = crate::Prefs {
        kept: vessel_app_view::prefs::Kept::File {
            path: path.display().to_string(),
        },
        agent: Some("claude-code".parse().unwrap()),
        ..crate::Prefs::default()
    };
    prefs.keep_choices("laptop", &"claude-code".parse().unwrap(), &[choice()]);
    prefs
}
async fn answer(events: &mut tokio::sync::mpsc::Receiver<crate::Event>) -> FormAnswer {
    let event = phase("preference form receipt", events.recv())
        .await
        .unwrap();
    let crate::Event::Schedules(Message::FormReplied(answer)) = event else {
        panic!("not a form receipt");
    };
    answer
}
#[tokio::test]
async fn failed_real_preference_write_returns_original_form_and_sends_no_rpc() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("controlled preference peer", peer::controlled("laptop")).await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let request = request(context, &mut Tickets::default(), target());
        let path = home.0.join("prefs.json");
        std::fs::create_dir(&path).unwrap();
        let prefs = prefs(&path);
        let save_path = path.clone();
        let persisted = ordered_preferences(move || prefs.save_at_for_test(&save_path));
        phase(
            "failed file before schedule",
            state.schedule_remembered(
                state
                    .selected_connection(request.connection())
                    .map_err(ScheduleError::NotSelected),
                request.clone(),
                persisted,
            ),
        )
        .await;
        let answer = answer(&mut events).await;
        assert_eq!(answer.request, request);
        assert!(matches!(answer.result, Err(ScheduleError::Preferences)));
        assert!(matches!(
            peer.requests.as_mut().unwrap().try_recv(),
            Err(TryRecvError::Empty)
        ));
        assert!(path.is_dir());
        assert!(!path.with_extension("json.tmp").exists());
        assert_eq!(
            ScheduleError::Preferences.editor_failure(request.operation()),
            Failure::Preferences
        );
        assert!(!Failure::Preferences.retryable());
    })
    .await;
}
#[tokio::test]
async fn real_preference_file_is_committed_before_the_original_create_frame() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase("controlled saved preference peer", peer::controlled("laptop")).await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let request = request(context, &mut Tickets::default(), target());
        let path = home.0.join("settings").join("prefs.json");
        let prefs = prefs(&path);
        let save_path = path.clone();
        phase("save file then schedule", state.schedule_remembered(state.selected_connection(request.connection()).map_err(ScheduleError::NotSelected), request.clone(), ordered_preferences(move || prefs.save_at_for_test(&save_path)))).await;
        let frame = phase("create after file commit", peer.requests.as_mut().unwrap().recv()).await.unwrap();
        let vessel_proto::Call::CreateSchedule(sent) = &frame.call else { panic!("wrong RPC"); };
        assert!(matches!(request.operation(), io::Operation::Save(save) if **save == Save::Create(sent.clone())));
        let saved: crate::Prefs = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.choices("laptop", &"claude-code".parse().unwrap()), Some([choice()].as_slice()));
        assert_private(&path);
        frame.answer.send(vessel_proto::Outcome::Err(WireError::new(ErrorCode::Unsupported, "synthetic preference-order refusal"))).unwrap();
        let answer = answer(&mut events).await;
        assert_eq!(answer.request, request);
        assert!(matches!(answer.result, Err(ScheduleError::Call(_))));
    }).await;
}
#[tokio::test]
async fn heartbeat_or_retired_admission_cannot_poll_the_preference_writer() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = phase(
            "controlled refused preference peer",
            peer::controlled("laptop"),
        )
        .await;
        let context = super::admission::insert(&mut state, "laptop", &mut peer);
        let mut tickets = Tickets::default();
        let heartbeat = ScheduleTarget::Session {
            session: vessel_proto::SessionId::random(),
        };
        for target in [heartbeat, target()] {
            let retired = matches!(target, ScheduleTarget::NewSession { .. });
            if retired {
                state.clients.remove("laptop").unwrap().retire();
            }
            let request = request(context.clone(), &mut tickets, target);
            let polled = Arc::new(AtomicBool::new(false));
            let saved = Arc::clone(&polled);
            let persistence = async move {
                saved.store(true, Ordering::SeqCst);
                SaveOutcome::Saved
            };
            phase(
                "refuse before preference effect",
                state.schedule_remembered(
                    state
                        .selected_connection(request.connection())
                        .map_err(ScheduleError::NotSelected),
                    request.clone(),
                    persistence,
                ),
            )
            .await;
            assert!(!polled.load(Ordering::SeqCst));
            let answer = answer(&mut events).await;
            assert_eq!(answer.request, request);
            assert!(matches!(
                answer.result,
                Err(ScheduleError::Preferences | ScheduleError::NotSelected(Refusal::Changed))
            ));
            let received = peer.requests.as_mut().unwrap().try_recv();
            if retired {
                // Retiring the last admitted client may also close the peer's
                // request pipe. Neither Empty nor Disconnected contains an RPC.
                assert!(received.is_err());
            } else {
                assert!(matches!(received, Err(TryRecvError::Empty)));
            }
        }
    })
    .await;
}

#[cfg(unix)]
fn assert_private(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt as _;
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}
#[cfg(not(unix))]
fn assert_private(_path: &std::path::Path) {}

#[test]
fn remembered_payload_redacts_preferences_and_preserves_the_refusal_envelope() {
    let context = vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            vessel_app_view::connection::MachineKey::from_book("laptop"),
            vessel_proto::NodeId::random(),
        )
        .unwrap();
    let request = request(context, &mut Tickets::default(), target());
    let mut preferences = crate::Prefs::default();
    let _old = preferences.folders.insert(
        "private-prefs-canary".into(),
        "/private-preference-path-canary".into(),
    );
    let wrapped = super::super::Request::remembering(request.clone(), preferences);
    let shown = format!("{wrapped:?}");
    assert!(!shown.contains("private-prefs-canary"));
    assert!(!shown.contains("/private-preference-path-canary"));
    assert!(shown.contains("<redacted>"));
    assert_eq!(wrapped.connection(), request.connection());
    let Message::FormReplied(answer) = wrapped.refused(ScheduleError::Preferences) else {
        panic!("not form");
    };
    assert_eq!(answer.request, request);
    assert!(matches!(answer.result, Err(ScheduleError::Preferences)));
}
