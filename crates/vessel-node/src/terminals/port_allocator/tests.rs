//! The real selection function against typed declarations and loopback sockets.

use std::error::Error as _;
use std::path::Path;

use vessel_proto::ProjectId;
use vessel_proto::scripts::ScriptName;

use super::*;

fn root(folder: &Path) -> ScriptRoot {
    ScriptRoot {
        project: ProjectId::random(),
        root: folder.to_path_buf(),
        project_root: folder.to_path_buf(),
        worktree: None,
        branch: None,
    }
}

fn entry(port: Option<ServicePort>) -> ScriptEntry {
    ScriptEntry::new(
        "web".parse().unwrap(),
        ScriptKind::Service,
        "serve".parse().unwrap(),
        None,
        port,
    )
    .unwrap()
}

#[test]
fn range_order_wraps_once_and_includes_both_tcp_boundaries() {
    let range: PortRange = "7000-7002".parse().unwrap();
    assert_eq!(
        candidates(range, RangeOffset::from_seed([1, 0]))
            .unwrap()
            .collect::<Vec<_>>(),
        [7001, 7002, 7000]
    );
    let full = PortRange::new(ServicePort::MIN, ServicePort::MAX).unwrap();
    let order: Vec<_> = candidates(full, RangeOffset::from_seed(u16::MAX.to_le_bytes()))
        .unwrap()
        .collect();
    assert_eq!(order.len(), usize::from(ServicePort::MAX.get()));
    assert_eq!(order.first(), Some(&ServicePort::MIN.get()));
    assert_eq!(order.last(), Some(&ServicePort::MAX.get()));
    assert_eq!(
        order.iter().copied().collect::<HashSet<_>>().len(),
        order.len()
    );
}

#[tokio::test]
async fn a_busy_or_reserved_singleton_range_is_refused_with_its_actual_cause() {
    let held = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = ServicePort::try_from(held.local_addr().unwrap().port()).unwrap();
    let range = PortRange::new(port, port).unwrap();
    let cancel = CancellationToken::new();
    let error = from_range(
        range,
        RangeOffset::from_seed([0, 0]),
        &HashSet::new(),
        &cancel,
    )
    .await
    .unwrap_err();
    assert!(matches!(&error, AllocationError::Exhausted { range: found, .. } if *found == range));
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::AddrInUse
    );
    let error = from_range(
        range,
        RangeOffset::from_seed([0, 0]),
        &HashSet::from([port]),
        &cancel,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AllocationError::Exhausted { last: None, .. }
    ));
}

#[tokio::test]
async fn explicit_ports_precede_both_choices_and_global_dynamic_ports_do_not_override_them() {
    let temp = tempfile::tempdir().unwrap();
    let root = root(temp.path());
    let port = ServicePort::try_from(3210_u16).unwrap();
    let entry = entry(Some(port));
    let options = ServicePorts::new(
        Some("7000-7002".parse().unwrap()),
        Some("missing".parse().unwrap()),
    )
    .unwrap();
    let empty = HashSet::new();
    let dynamic = HashSet::from([port]);
    let cancel = CancellationToken::new();
    let request = || Allocation {
        root: &root,
        entry: &entry,
        options: Some(&options),
        reserved: &empty,
        dynamic: &dynamic,
        offset: RangeOffset::from_seed([0, 0]),
        removed: &[],
        cancel: &cancel,
    };
    assert_eq!(allocate(request()).await.unwrap(), port);
    let mut duplicate = request();
    duplicate.reserved = &dynamic;
    assert!(
        matches!(allocate(duplicate).await, Err(AllocationError::Duplicate(found)) if found == port)
    );
}

#[tokio::test]
async fn a_missing_helper_does_not_fall_back_to_the_range() {
    let temp = tempfile::tempdir().unwrap();
    let root = root(temp.path());
    let entry = entry(None);
    let options = ServicePorts::new(
        Some("7000-7002".parse().unwrap()),
        Some("missing".parse().unwrap()),
    )
    .unwrap();
    let empty = HashSet::new();
    let cancel = CancellationToken::new();
    let result = allocate(Allocation {
        root: &root,
        entry: &entry,
        options: Some(&options),
        reserved: &empty,
        dynamic: &empty,
        offset: RangeOffset::from_seed([0, 0]),
        removed: &[],
        cancel: &cancel,
    })
    .await;
    assert!(matches!(
        result,
        Err(AllocationError::Program(PortScriptError::Path(_)))
    ));
}

#[tokio::test]
async fn no_block_allocates_a_nonzero_kernel_port_and_plain_scripts_never_allocate() {
    let temp = tempfile::tempdir().unwrap();
    let root = root(temp.path());
    let service = entry(None);
    let empty = HashSet::new();
    let cancel = CancellationToken::new();
    let mut request = Allocation {
        root: &root,
        entry: &service,
        options: None,
        reserved: &empty,
        dynamic: &empty,
        offset: RangeOffset::from_seed([0, 0]),
        removed: &[],
        cancel: &cancel,
    };
    let plain = ScriptEntry::new(
        "once".parse::<ScriptName>().unwrap(),
        ScriptKind::Script,
        "once".parse().unwrap(),
        None,
        None,
    )
    .unwrap();
    request.entry = &plain;
    assert!(matches!(
        allocate(request).await,
        Err(AllocationError::NotService)
    ));
    let port = allocate(Allocation {
        root: &root,
        entry: &service,
        options: None,
        reserved: &empty,
        dynamic: &empty,
        offset: RangeOffset::from_seed([0, 0]),
        removed: &[],
        cancel: &cancel,
    })
    .await
    .unwrap();
    assert!((ServicePort::MIN..=ServicePort::MAX).contains(&port));
    cancel.cancel();
    let result = allocate(Allocation {
        root: &root,
        entry: &service,
        options: None,
        reserved: &empty,
        dynamic: &empty,
        offset: RangeOffset::from_seed([0, 0]),
        removed: &[],
        cancel: &cancel,
    })
    .await;
    assert!(matches!(result, Err(AllocationError::Cancelled)));
}

#[cfg(unix)]
#[tokio::test]
async fn the_helper_wins_over_range_is_not_availability_probed_and_cannot_return_a_reserved_port() {
    struct Sandbox;
    impl Sandbox {
        fn executable(path: &Path, text: &str) {
            vessel_testkit::write_executable(path, text).unwrap();
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = root(temp.path());
    let held = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = ServicePort::try_from(held.local_addr().unwrap().port()).unwrap();
    Sandbox::executable(
        &root.root.join("port-tool"),
        &format!("#!/bin/sh\nprintf '{port}\\n'\n"),
    );
    let other = if port == ServicePort::MIN {
        ServicePort::MAX
    } else {
        ServicePort::MIN
    };
    let options = ServicePorts::new(
        Some(PortRange::new(other, other).unwrap()),
        Some("port-tool".parse().unwrap()),
    )
    .unwrap();
    let entry = entry(None);
    let empty = HashSet::new();
    let occupied = HashSet::from([port]);
    let cancel = CancellationToken::new();
    let request = || Allocation {
        root: &root,
        entry: &entry,
        options: Some(&options),
        reserved: &empty,
        dynamic: &empty,
        offset: RangeOffset::from_seed([0, 0]),
        removed: &[],
        cancel: &cancel,
    };
    assert_eq!(allocate(request()).await.unwrap(), port);
    for (local, global) in [(&occupied, &empty), (&empty, &occupied)] {
        let mut reserved = request();
        reserved.reserved = local;
        reserved.dynamic = global;
        assert!(matches!(allocate(reserved).await,
            Err(AllocationError::HelperReserved(found)) if found == port));
    }
}
