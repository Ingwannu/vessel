//! Scope/matcher values alone never authorize Preview admission.

use super::*;
use crate::connection_life::ConnectionLife;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio_util::sync::CancellationToken;
use vessel_proto::{SessionId, WorkspaceId, workspace::ServicePort};

#[test]
fn literal_origins_keep_exact_scheme_address_and_port() {
    for ip in [Ipv4Addr::LOCALHOST.into(), Ipv6Addr::LOCALHOST.into()] {
        let address = SocketAddr::new(ip, ServicePort::MAX.get());
        let http = PreviewOrigin::new(PreviewScheme::Http, address).unwrap();
        let https = PreviewOrigin::new(PreviewScheme::Https, address).unwrap();
        assert_eq!(http.endpoint(), address);
        assert_eq!(http.scheme(), PreviewScheme::Http);
        assert_ne!(http, https);
    }
    let zero = ServicePort::MIN
        .get()
        .checked_sub(ServicePort::MIN.get())
        .unwrap();
    assert!(matches!(
        PreviewOrigin::new(
            PreviewScheme::Http,
            SocketAddr::from((Ipv4Addr::LOCALHOST, zero))
        ),
        Err(PreviewError::Port(_))
    ));
    assert!(matches!(
        PreviewOrigin::new(
            PreviewScheme::Http,
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, ServicePort::MIN.get()))
        ),
        Err(PreviewError::NotLoopback)
    ));
}

#[test]
fn observers_do_not_keep_connection_alive_and_transport_close_is_independent() {
    let life = ConnectionLife::default();
    let transport = CancellationToken::new();
    let scope = PreviewScope {
        workspace: WorkspaceId::random(),
        opening_session: SessionId::random(),
    };
    let context = PreviewContext::from_observers(scope, &life, transport.clone());
    assert_eq!(context.scope, scope);
    assert_eq!(context.owner, life.owner());
    assert!(!context.ended());
    drop(life);
    assert!(context.ended());
    let life = ConnectionLife::default();
    let second = PreviewContext::from_observers(scope, &life, transport.clone());
    assert_ne!(context.owner, second.owner);
    assert!(!second.ended());
    transport.cancel();
    assert!(second.ended());
    assert!(!life.closed().is_cancelled());
}

#[test]
fn port_refusal_preserves_its_validating_source() {
    use std::error::Error as _;
    let zero = ServicePort::MIN
        .get()
        .checked_sub(ServicePort::MIN.get())
        .unwrap();
    let Err(error @ PreviewError::Port(_)) = PreviewOrigin::new(
        PreviewScheme::Http,
        SocketAddr::from((Ipv4Addr::LOCALHOST, zero)),
    ) else {
        panic!("zero was accepted")
    };
    assert!(error.source().is_some());
}
