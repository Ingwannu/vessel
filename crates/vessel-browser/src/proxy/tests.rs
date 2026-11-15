//! Own ephemeral loopback fixtures only; these are not Chromium qualification.

use super::*;
use std::io::ErrorKind;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);

#[tokio::test]
async fn a_live_private_listener_is_refused_for_http_and_connect() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let canary = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        canary.set_nonblocking(true).unwrap();
        let canary_address = canary.local_addr().unwrap();
        // Positive control: the listener is reachable by an ordinary local dial.
        let direct = TcpStream::connect(canary_address).await.unwrap();
        let (accepted, _) = canary.accept().unwrap();
        drop(direct);
        drop(accepted);

        let proxy = PublicProxy::start().await.unwrap();
        for request in [
            format!("GET http://{canary_address}/ HTTP/1.1\r\nHost: ignored.test\r\nConnection: close\r\n\r\n"),
            format!("CONNECT {canary_address} HTTP/1.1\r\nHost: ignored.test\r\nConnection: close\r\n\r\n"),
        ] {
            let mut client = TcpStream::connect(proxy.endpoint()).await.unwrap();
            client.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            let _bytes = client.read_to_string(&mut response).await.unwrap();
            assert!(response.starts_with("HTTP/1.1 403"), "{response}");
            assert!(!response.contains("ignored.test"));
            assert_eq!(canary.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
        }
        proxy.close().await.unwrap();
    }).await.unwrap();
}

#[tokio::test]
async fn dropping_the_profile_cancels_an_incomplete_client_connection() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let proxy = PublicProxy::start().await.unwrap();
        let mut client = TcpStream::connect(proxy.endpoint()).await.unwrap();
        client.write_all(b"GET ").await.unwrap();
        proxy.close().await.unwrap();
        let mut data = Vec::new();
        // EOF or reset are both closed transports; no child route can survive.
        if let Err(error) = client.read_to_end(&mut data).await {
            assert!(matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::UnexpectedEof
            ));
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn profile_close_cannot_finish_or_replace_capacity_while_preparation_is_alive() {
    use crate::preparation::{Preparation, PreparationError};
    use std::sync::{Arc, mpsc as blocking};
    use tokio::sync::{Semaphore, oneshot};
    use tokio_util::sync::CancellationToken;

    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let gate = Arc::new(Semaphore::new(1));
        let cancel = CancellationToken::new();
        let preparation = Preparation::with_gate(cancel.clone(), Arc::clone(&gate));
        let proxy = PublicProxy::start_scope(cancel.clone(), preparation.clone())
            .await
            .unwrap();
        let (started, entered) = oneshot::channel();
        let (release, held) = blocking::channel();
        let worker_scope = preparation.clone();
        let caller = tokio::spawn(async move {
            worker_scope
                .run(move || {
                    let _gone = started.send(());
                    held.recv().unwrap();
                })
                .await
        });
        entered.await.unwrap();
        let closing = tokio::spawn(proxy.close());
        cancel.cancelled().await;
        assert!(matches!(
            caller.await.unwrap(),
            Err(PreparationError::Closed)
        ));
        assert!(!closing.is_finished());
        let replacement = Preparation::with_gate(CancellationToken::new(), Arc::clone(&gate));
        assert!(matches!(
            replacement.run(|| ()).await,
            Err(PreparationError::Busy)
        ));
        release.send(()).unwrap();
        closing.await.unwrap().unwrap();
        replacement.run(|| ()).await.unwrap();
        replacement.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_drain_wait_keeps_the_actual_preparation_and_can_resume() {
    use crate::preparation::{Preparation, PreparationError};
    use std::sync::{Arc, mpsc as blocking};
    use tokio::sync::{Semaphore, oneshot};
    use tokio_util::sync::CancellationToken;
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let gate = Arc::new(Semaphore::new(1));
        let cancel = CancellationToken::new();
        let preparation = Preparation::with_gate(cancel.clone(),Arc::clone(&gate));
        let proxy = PublicProxy::start_scope(cancel.clone(),preparation.clone()).await.unwrap();
        let (started,entered) = oneshot::channel();
        let (release,held) = blocking::channel();
        let worker = tokio::spawn(async move {
            preparation.run(move || { let _notified=started.send(()); held.recv().unwrap(); }).await
        });
        entered.await.unwrap();
        let mut closing = proxy.begin_close();
        cancel.cancelled().await;
        assert!(matches!(worker.await.unwrap(),Err(PreparationError::Closed)));
        {
            let wait = closing.wait();
            tokio::pin!(wait);
            tokio::select! {
                biased;
                _finished = &mut wait => panic!("cleanup completed while the real closure was held"),
                () = tokio::task::yield_now() => {},
            }
            // This outer caller stops waiting, but ClosingProxy still owns its JoinHandle.
        }
        let replacement = Preparation::with_gate(CancellationToken::new(),Arc::clone(&gate));
        assert!(matches!(replacement.run(|| ()).await,Err(PreparationError::Busy)));
        release.send(()).unwrap();
        let _actual = closing.wait().await.unwrap();
        let _same_completed_scope = closing.wait().await.unwrap();
        replacement.run(|| ()).await.unwrap();
        replacement.finish().await;
    }).await.unwrap();
}

#[tokio::test]
async fn failed_actual_drain_never_becomes_success_on_a_second_wait() {
    let cancel = tokio_util::sync::CancellationToken::new();
    let actor = tokio::spawn(async { Err(ProxyError::Unconfirmed) });
    let mut closing = ClosingProxy::new(Some(actor), cancel);
    assert!(matches!(closing.wait().await, Err(ProxyError::Unconfirmed)));
    assert!(matches!(closing.wait().await, Err(ProxyError::Unconfirmed)));
}
