//! The file, rather than a later launcher, owns peer environment collisions.

use super::*;
use vessel_proto::scripts::ServiceEnvName;

#[test]
fn service_environment_collisions_refuse_the_whole_file_before_setup_can_run() {
    for (first, second) in [
        ("web-app", "web_app"),
        ("web.app", "web-app"),
        ("web", "web-"),
        ("WEB", "web"),
    ] {
        let text = format!(
            "[worktree]\nsetup = 'must not run'\n[services.\"{first}\"]\ncommand = 'a'\n[services.\"{second}\"]\ncommand = 'b'\n"
        );
        let error = ProjectFile::parse(text.as_bytes()).unwrap_err();
        match error {
            ProjectFileError::Scripts(ScriptsError::Environment {
                first: a,
                second: b,
                environment,
            }) => {
                let mut actual = [a.as_str(), b.as_str()];
                actual.sort();
                let mut expected = [first, second];
                expected.sort();
                assert_eq!(actual, expected);
                assert_eq!(environment, ServiceEnvName::of(&a));
                assert_eq!(environment, ServiceEnvName::of(&b));
            }
            other => panic!("expected environment collision, got {other:?}"),
        }
    }
}

#[test]
fn plain_scripts_do_not_reserve_service_environment_names() {
    for text in [
        "[scripts.web-app]\ncommand='once'\n[services.web_app]\ncommand='serve'\n",
        "[scripts.web-app]\ncommand='once'\n[scripts.web_app]\ncommand='twice'\n",
    ] {
        let file = ProjectFile::parse(text.as_bytes()).unwrap();
        assert_eq!(file.scripts.len(), 2);
    }
}
