//! `agent.install` end to end (protocol 11, ADR 0032): the vendor's
//! documented command run on the node, its output read back, and who may
//! run one when. The installer is the testkit's fake tool under `npm`'s
//! name: nothing real is installed.

use vessel_proto::agents::{InstallAgent, InstallPart, InstallState, ReadInstall};
use vessel_proto::{ErrorCode, HarnessKind, InstallId};
use vessel_testkit::Fake;

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

/// The fake tool as `npm` on the fixture's search path.
fn place_npm(fixture: &Fixture) {
    let tool = Fake::Tool.path().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&tool, fixture.bin().join("npm")).unwrap();
    #[cfg(windows)]
    drop(vessel_testkit::write_cmd_shim(fixture.bin(), "npm", &tool).unwrap());
}

fn install(agent: HarnessKind, part: InstallPart) -> InstallAgent {
    InstallAgent { agent, part }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_install_runs_the_vendors_command_and_is_read_back() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;

        // No installer on the search path.
        assert_eq!(
            code(
                owner
                    .call(install(HarnessKind::Codex, InstallPart::Harness))
                    .await
            ),
            ErrorCode::HarnessNotInstalled
        );
        place_npm(&fixture);
        // Cursor's installer is a script piped to a shell (ADR 0058): shown
        // by `agent.list`, never run by the node.
        assert_eq!(
            code(
                owner
                    .call(install(HarnessKind::Cursor, InstallPart::Harness))
                    .await
            ),
            ErrorCode::Unsupported
        );
        // Gemini CLI speaks ACP itself: it has no adapter.
        assert_eq!(
            code(
                owner
                    .call(install(HarnessKind::Gemini, InstallPart::Adapter))
                    .await
            ),
            ErrorCode::InvalidRequest
        );

        let job = owner
            .call(install(HarnessKind::Codex, InstallPart::Harness))
            .await
            .unwrap();
        assert_eq!(job.command, "npm install -g @openai/codex");
        let (mut from, mut printed) = (0, Vec::new());
        let state = loop {
            let page = owner
                .call(ReadInstall {
                    job: job.id,
                    from,
                    wait: Some(30),
                })
                .await
                .unwrap();
            from = page.next;
            printed.extend_from_slice(&page.bytes.into_bytes());
            if page.state != InstallState::Running {
                break page.state;
            }
        };
        assert_eq!(state, InstallState::Succeeded);
        let printed = String::from_utf8_lossy(&printed);
        assert!(printed.contains("install -g @openai/codex"), "{printed}");
        // In the user's home: the fake writes its log there.
        let log = std::fs::read_to_string(fixture.user_home().join("fake-tool.log")).unwrap();
        assert!(log.contains("install -g @openai/codex"), "{log}");
        assert_eq!(
            code(
                owner
                    .call(ReadInstall {
                        job: InstallId::random(),
                        from: 0,
                        wait: None,
                    })
                    .await
            ),
            ErrorCode::NotFound
        );

        // Not beside a live session of that agent, and never for an agent.
        let mut running = structured(&owner, &fixture).await;
        assert_eq!(
            code(
                owner
                    .call(install(HarnessKind::ClaudeCode, InstallPart::Adapter))
                    .await
            ),
            ErrorCode::Conflict
        );
        let (_program, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            code(
                agent
                    .call(install(HarnessKind::Codex, InstallPart::Harness))
                    .await
            ),
            ErrorCode::Forbidden
        );
        node.stop().await.unwrap();
    })
    .await;
}
