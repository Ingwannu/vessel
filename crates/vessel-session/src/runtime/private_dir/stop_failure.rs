//! A refused OS signal is injected only at the actor's stop boundary. The
//! process, profile linking and attempted replacement remain real fake runs.

use super::*;

fn profiles(root: &Path) -> (Profile, Profile) {
    let text = format!(
        "[profiles.before]\nharness='claude'\nenv={{CLAUDE_CONFIG_DIR='{before}'}}\n\
         [profiles.after]\nharness='claude'\nenv={{CLAUDE_CONFIG_DIR='{after}'}}\n",
        before = root.join("before").display(),
        after = root.join("after").display(),
    );
    let profiles = parse_profiles(&text, Some(root)).unwrap();
    let named = |name: &str| profiles.get(&name.parse().unwrap()).cloned().unwrap();
    (named("before"), named("after"))
}

async fn prepared(fixture: &Fixture) -> (SessionHandle, ProfileMove) {
    let (before, after) = profiles(fixture.dir.path());
    let env = fixture.runtime.config.harness_env.clone();
    let from_home = env.home(&before).unwrap();
    let to_home = env.home(&after).unwrap();
    let handle = fixture.running(Some(before)).await;
    let conversation = handle.info().harness_session.unwrap();
    let project = claude_project(&from_home, &fixture.work()).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join(format!("{conversation}.jsonl")), "{}\n").unwrap();
    std::fs::create_dir_all(&to_home).unwrap();
    (
        handle,
        ProfileMove {
            profile: after,
            limits: crate::LimitAssociation::Unavailable,
            from_home,
            to_home,
            mcp_servers: ResolvedMcpServers::new(),
            notices: Vec::new(),
            force: false,
            model_proxy: None,
        },
    )
}

async fn refuse_next_stop(handle: &SessionHandle) {
    let control = handle.0.control.clone().unwrap();
    let (ready, answer) = oneshot::channel();
    control
        .send(Control::RefuseNextStop {
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied).into(),
            ready,
        })
        .await
        .unwrap();
    answer.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refuses_restart_after_a_signal_failure() {
    within(async {
        let fixture = Fixture::new();
        let (handle, order) = prepared(&fixture).await;
        let before = handle.info();
        let pid = handle.pid();
        assert!(pid.is_some());
        refuse_next_stop(&handle).await;
        let outcome = fixture.runtime.move_profile(handle.id(), order).await;
        let after = handle.info();
        let after_pid = handle.pid();
        let events = handle.follow(EventSeq::ZERO).unwrap().cached;
        let responsive = handle.options().await;
        // Cleanup also happens in the old, incorrectly successful path.
        handle.kill();
        let _ended = handle.ended().await;
        let error = outcome.unwrap_err();
        assert_eq!(error.code(), vessel_proto::ErrorCode::Internal);
        let crate::SessionError::Move(crate::MoveError::Signal { signal, source }) = error else {
            panic!("expected the typed stop-signal refusal");
        };
        assert_eq!(signal, vessel_agent::Signal::Terminate);
        assert!(source.is_refused());
        assert_eq!(after_pid, pid, "the original process must remain");
        assert_eq!(after.profile, before.profile);
        assert_eq!(after.harness_session, before.harness_session);
        assert!(after.status.is_live());
        assert!(responsive.is_ok(), "the original agent still answers");
        assert!(
            !events
                .iter()
                .any(|record| matches!(record.event, SessionEvent::ProfileChanged { .. }))
        );
    })
    .await;
}
