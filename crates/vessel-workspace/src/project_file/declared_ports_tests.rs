//! A value's port is covered before the later file decoder can produce it.

use super::*;
use vessel_proto::scripts::ScriptKind;
use vessel_proto::workspace::ServicePort;

fn service(port: Option<ServicePort>) -> ScriptEntry {
    ScriptEntry::new(
        "dev".parse().unwrap(),
        ScriptKind::Service,
        "serve-approval-port".parse().unwrap(),
        None,
        port,
    )
    .unwrap()
}

#[test]
fn a_declared_port_uses_v3_and_a_port_only_change_needs_approval_again() {
    let mut file = ProjectFile {
        scripts: vec![service(None)],
        ..ProjectFile::default()
    };
    let legacy = file.digest();
    file.scripts = vec![service(Some(ServicePort::try_from(3210_u16).unwrap()))];
    let declared = file.digest();
    assert_ne!(declared, legacy);
    assert_eq!(
        declared.as_str(),
        "edac2deb749e68fd95dea0b585e52862c5f73fe46e6c68dd3fd4f38c21241cd2"
    );
    file.scripts = vec![service(Some(ServicePort::try_from(3211_u16).unwrap()))];
    assert_ne!(file.digest(), declared);
}

#[test]
fn toml_ports_survive_decoding_and_enter_the_existing_approval_digest() {
    let sample = ServicePort::try_from(3210_u16).unwrap();
    for port in [ServicePort::MIN, sample, ServicePort::MAX] {
        let text = format!("[services.dev]\ncommand = \"serve-approval-port\"\nport = {port}\n");
        let parsed = ProjectFile::parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.scripts.first().unwrap().port(), Some(port));
        let expected = ProjectFile {
            scripts: vec![service(Some(port))],
            ..ProjectFile::default()
        };
        assert_eq!(parsed, expected);
        assert_eq!(parsed.digest(), expected.digest());
    }
}

#[test]
fn a_port_on_a_plain_script_refuses_the_whole_file() {
    use vessel_proto::scripts::InvalidScriptEntry;
    let text =
        b"[worktree]\nsetup = \"do-not-run\"\n[scripts.dev]\ncommand = \"serve\"\nport = 3210\n";
    assert!(matches!(
        ProjectFile::parse(text),
        Err(ProjectFileError::Scripts(ScriptsError::Declaration(
            InvalidScriptEntry::PortOnScript
        )))
    ));
}

#[test]
fn invalid_port_numbers_are_refused_before_a_file_can_be_approved() {
    use std::error::Error as _;
    let below = i64::from(ServicePort::MIN.get()).saturating_sub(i64::from(ServicePort::MIN.get()));
    let above = i64::from(ServicePort::MAX.get()).saturating_add(i64::from(ServicePort::MIN.get()));
    for number in [below, above] {
        let text = format!("[services.dev]\ncommand = \"serve\"\nport = {number}\n");
        let error = ProjectFile::parse(text.as_bytes()).unwrap_err();
        let ProjectFileError::Scripts(ref declaration) = error else {
            panic!("typed declaration error");
        };
        assert!(
            declaration
                .source()
                .unwrap()
                .is::<vessel_proto::workspace::InvalidServicePort>()
        );
        assert!(matches!(declaration, ScriptsError::Port { .. }));
    }
    for number in ["3210.5", "\"3210\"", "true"] {
        let text = format!("[services.dev]\ncommand = \"serve\"\nport = {number}\n");
        assert!(ProjectFile::parse(text.as_bytes()).is_err(), "{number}");
    }
}
