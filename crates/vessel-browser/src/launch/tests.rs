//! Launch-data invariants only, never a fake/qualified browser process.
use super::*;
use crate::profile::Profile;
use vessel_proc::{EnvValue, Environment};
#[test]
fn candidate_argv_never_interpolates_page_data_and_only_explicit_environment_survives() {
    let parent = tempfile::tempdir_in("/tmp").unwrap();
    let profile =
        Profile::prepare(&parent.path().join("profile"), &parent.path().join("tmp")).unwrap();
    let proxy = ProxyEndpoint::try_from("127.0.0.1:8123".parse::<SocketAddr>().unwrap()).unwrap();
    // Private value constructor is test-only data, not a runtime qualification.
    let plan = Plan::build(profile, proxy, Candidate::FIXTURE);
    assert!(!format!("{plan:?}").contains(parent.path().to_str().unwrap()));
    let (command, profile) = plan.into_supervised_parts();
    assert_eq!(command.diagnostic_policy(), CommandDiagnostics::Redacted);
    assert!(!format!("{command:?}").contains(profile.root().to_str().unwrap()));
    assert_eq!(command.environment, Environment::Clear);
    assert_eq!(command.program, PathBuf::from(FIXED_SHELL));
    assert_eq!(command.args.first().unwrap(), "-c");
    assert_eq!(command.args[1], FIXED_SOURCE);
    assert_eq!(command.args[2], INERT_LABEL);
    assert!(!command.args.iter().any(|arg| arg == "--no-sandbox"));
    assert!(
        command
            .args
            .iter()
            .any(|arg| arg == "--remote-debugging-pipe")
    );
    assert!(
        !command
            .args
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with("--remote-debugging-port"))
    );
    assert!(
        matches!(command.env_change("HOME"),Some(EnvValue::Set(value)) if value==profile.home().as_os_str())
    );
    for name in [
        "SSH_AUTH_SOCK",
        "DBUS_SESSION_BUS_ADDRESS",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
    ] {
        assert_eq!(command.env_change(name), None);
    }
    // Inspector's refusal on the build VM is expected when the optional exact
    // candidate is absent. It must never invent a ready engine there.
    let inspected = Plan::inspect(profile, proxy);
    if let Ok(plan) = inspected {
        let (_command, profile) = plan.into_supervised_parts();
        profile.release().unwrap();
    }
    for refused in ["0.0.0.0:8123", "127.0.0.1:0", "[::1]:8123"] {
        assert!(ProxyEndpoint::try_from(refused.parse::<SocketAddr>().unwrap()).is_err());
    }
}
use std::path::PathBuf;

#[tokio::test]
async fn a_different_actual_owned_child_cannot_inherit_the_browser_candidate_identity() {
    const FIXTURE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
    tokio::time::timeout(FIXTURE_TIMEOUT, async {
        let parent = tempfile::tempdir_in("/tmp").unwrap();
        let profile =
            Profile::prepare(&parent.path().join("profile"), &parent.path().join("tmp")).unwrap();
        let proxy =
            ProxyEndpoint::try_from("127.0.0.1:8123".parse::<SocketAddr>().unwrap()).unwrap();
        let plan = Plan::build(profile, proxy, Candidate::FIXTURE);
        let helper = CommandSpec::new(FIXED_SHELL)
            .env_clear()
            .arg("-c")
            .arg("read -r fixture");
        let mut spawned = vessel_proc::spawn(&helper).unwrap();
        assert!(matches!(
            plan.verify_started(&spawned.child),
            Err(LaunchError::Identity)
        ));
        drop(spawned.stdin);
        let _exited = spawned.child.wait().await.unwrap();
        let (_command, profile) = plan.into_supervised_parts();
        profile.release().unwrap();
    })
    .await
    .unwrap();
}
