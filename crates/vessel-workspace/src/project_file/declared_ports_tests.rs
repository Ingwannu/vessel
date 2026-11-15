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
