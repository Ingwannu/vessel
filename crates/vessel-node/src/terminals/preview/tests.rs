//! Scope/matcher values alone never authorize Preview admission.

use super::*;
use super::{budget::*, fixtures};
use crate::connection_life::ConnectionLife;
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};
use tokio::net::TcpListener;
use tokio_util::task::TaskTracker;

mod lifetime;
mod transport;

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

const TEST_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

struct Rig {
    _connection: ConnectionLife,
    context: PreviewContext,
    origin: PreviewOrigin,
    service: PreviewService,
    control: fixtures::Control,
}

impl Rig {
    fn new(origin: PreviewOrigin) -> Self {
        let connection = ConnectionLife::default();
        let context = context(&connection);
        let (service, control) = fixtures::service(&context, origin);
        Self {
            _connection: connection,
            context,
            origin,
            service,
            control,
        }
    }

    async fn lease(&self) -> PreviewLease {
        self.service
            .admit(self.context.clone(), self.origin)
            .await
            .unwrap()
    }
}

fn context(connection: &ConnectionLife) -> PreviewContext {
    PreviewContext::from_observers(
        PreviewScope {
            workspace: WorkspaceId::random(),
            opening_session: SessionId::random(),
        },
        connection,
        CancellationToken::new(),
    )
}

fn origin() -> PreviewOrigin {
    PreviewOrigin::new(
        PreviewScheme::Http,
        SocketAddr::from((Ipv4Addr::LOCALHOST, ServicePort::MIN.get())),
    )
    .unwrap()
}

async fn listener() -> (TcpListener, PreviewOrigin) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let origin = PreviewOrigin::new(PreviewScheme::Http, listener.local_addr().unwrap()).unwrap();
    (listener, origin)
}

async fn pending<F: std::future::Future>(mut future: std::pin::Pin<&mut F>) {
    std::future::poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
}

#[tokio::test]
async fn unavailable_never_dials_or_reserves_physical_capacity() {
    let (listener, origin) = listener().await;
    let connection = ConnectionLife::default();
    let context = context(&connection);
    let service = PreviewService::unavailable(CancellationToken::new(), TaskTracker::new());
    assert!(matches!(
        service.admit(context, origin).await,
        Err(PreviewError::Unavailable(
            MissingPreviewProof::CanonicalAuthority
        ))
    ));
    assert_eq!(
        service.runtime.leases.available_permits(),
        MAX_PREVIEW_LEASES
    );
    assert_eq!(
        service.runtime.connections.available_permits(),
        MAX_PREVIEW_STREAMS
    );
    assert_eq!(
        service.runtime.preparations.available_permits(),
        MAX_PREVIEW_PREPARATIONS
    );
    pending(std::pin::pin!(listener.accept())).await;
}

#[tokio::test]
async fn each_command_refuses_foreign_scope_connection_and_exact_origin() {
    let rig = Rig::new(origin());
    let lease = rig.lease().await;
    lease.check(&rig.context, rig.origin).await.unwrap();
    let mut other = rig.context.clone();
    other.scope.workspace = WorkspaceId::random();
    assert!(matches!(
        lease.check(&other, rig.origin).await,
        Err(PreviewError::Scope)
    ));
    other = rig.context.clone();
    other.scope.opening_session = SessionId::random();
    assert!(matches!(
        lease.check(&other, rig.origin).await,
        Err(PreviewError::Scope)
    ));
    let foreign_connection = ConnectionLife::default();
    other = PreviewContext::from_observers(
        rig.context.scope,
        &foreign_connection,
        CancellationToken::new(),
    );
    assert!(matches!(
        lease.check(&other, rig.origin).await,
        Err(PreviewError::Connection)
    ));
    let https = PreviewOrigin::new(PreviewScheme::Https, rig.origin.endpoint()).unwrap();
    assert!(matches!(
        lease.check(&rig.context, https).await,
        Err(PreviewError::Origin)
    ));
    let other_port = PreviewOrigin::new(
        PreviewScheme::Http,
        SocketAddr::from((Ipv4Addr::LOCALHOST, ServicePort::MAX.get())),
    )
    .unwrap();
    assert!(matches!(
        lease.check(&rig.context, other_port).await,
        Err(PreviewError::Origin)
    ));
    let receipt: PreviewDrain = lease.revoke_and_drain().await;
    assert!(receipt.belongs_to(&lease));
}

#[tokio::test]
async fn capacities_are_shared_by_service_clones_and_errors_keep_the_bound() {
    use std::error::Error as _;
    let rig = Rig::new(origin());
    let mut leases = Vec::new();
    for _ in 0..MAX_PREVIEW_LEASES {
        leases.push(rig.lease().await);
    }
    let result = rig
        .service
        .clone()
        .admit(rig.context.clone(), rig.origin)
        .await;
    let Err(
        error @ PreviewError::Busy {
            budget: PreviewBudget::Leases,
            ..
        },
    ) = result
    else {
        panic!("lease budget was bypassed")
    };
    assert!(error.source().is_some());
    assert!(error.to_string().contains(&MAX_PREVIEW_LEASES.to_string()));
    for lease in leases {
        let _receipt = lease.revoke_and_drain().await;
    }
    assert_eq!(
        rig.service.runtime.leases.available_permits(),
        MAX_PREVIEW_LEASES
    );
    let lease = rig.lease().await;
    let another = rig.lease().await;
    let receipt = lease.revoke_and_drain().await;
    assert!(!receipt.belongs_to(&another));
    let _receipt = another.revoke_and_drain().await;
    assert!(Arc::ptr_eq(
        &rig.service.runtime,
        &rig.service.clone().runtime
    ));
}

#[test]
fn exact_ipv6_origin_preserves_scope_and_flow_instead_of_aliasing_metadata() {
    use std::net::SocketAddrV6;
    let endpoint = SocketAddrV6::new(Ipv6Addr::LOCALHOST, ServicePort::MIN.get(), 7, 9);
    let original = PreviewOrigin::new(PreviewScheme::Http, endpoint.into()).unwrap();
    let SocketAddr::V6(retained) = original.endpoint() else {
        panic!("IPv6 was replaced")
    };
    assert_eq!(retained.scope_id(), endpoint.scope_id());
    assert_eq!(retained.flowinfo(), endpoint.flowinfo());
    let plain = PreviewOrigin::new(
        PreviewScheme::Http,
        SocketAddr::from((Ipv6Addr::LOCALHOST, ServicePort::MIN.get())),
    )
    .unwrap();
    assert_ne!(original, plain);
    let mut other_scope = endpoint;
    other_scope.set_scope_id(endpoint.scope_id().checked_add(1).unwrap());
    assert_ne!(
        original,
        PreviewOrigin::new(PreviewScheme::Http, other_scope.into()).unwrap()
    );
}
