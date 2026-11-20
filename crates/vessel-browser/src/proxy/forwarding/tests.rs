//! Real controlled TCP peers below destination admission, not policy/engine proof.
use super::*;
use crate::preparation::Preparation;
use hyper::service::service_fn;
use std::{convert::Infallible, net::Ipv4Addr, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, oneshot},
    task::JoinHandle,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
const FIXTURE_CONNECTIONS: usize = 1;
struct Fixture {
    address: std::net::SocketAddr,
    upstream: TcpListener,
    context: Context,
    capacity: Arc<Semaphore>,
    frontend: JoinHandle<()>,
}
impl Fixture {
    async fn start() -> Self {
        let upstream = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let backend = upstream.local_addr().unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancel = CancellationToken::new();
        let capacity = Arc::new(Semaphore::new(FIXTURE_CONNECTIONS));
        let context = Context {
            cancel: cancel.clone(),
            tasks: TaskTracker::new(),
            preparation: Preparation::for_profile(cancel.clone()),
            permit: Arc::new(Arc::clone(&capacity).try_acquire_owned().unwrap()),
        };
        let serving = context.clone();
        let frontend = tokio::spawn(async move {
            let (client, _) = listener.accept().await.unwrap();
            let serving_cancel = serving.cancel.clone();
            let service = service_fn(move |request: Request<Incoming>| {
                let context = serving.clone();
                async move {
                    let destination = request::destination(&request).unwrap();
                    // Deliberate fixture-only boundary: controlled peer after
                    // admission. Product serve always obtains this via dial_public.
                    let stream = TcpStream::connect(backend).await.unwrap();
                    Ok::<_, Infallible>(connected(request, destination, stream, context).await)
                }
            });
            let mut builder = hyper::server::conn::http1::Builder::new();
            let _configured = builder
                .max_headers(super::super::MAX_PROXY_HEADERS)
                .max_buf_size(super::super::MAX_PROXY_HEADER_BYTES);
            tokio::select! {
                ()=serving_cancel.cancelled()=>{},
                _finished=builder.serve_connection(TokioIo::new(client),service).with_upgrades()=>{},
            }
        });
        Self {
            address,
            upstream,
            context,
            capacity,
            frontend,
        }
    }
    async fn stop(self) {
        self.context.cancel.cancel();
        self.frontend.await.unwrap();
        let _closed = self.context.tasks.close();
        self.context.tasks.wait().await;
        self.context.preparation.finish().await;
        drop(self.upstream);
        drop(self.context);
        assert_eq!(self.capacity.available_permits(), FIXTURE_CONNECTIONS);
    }
}
async fn headers(stream: &mut BufReader<TcpStream>) -> String {
    let mut result = String::new();
    loop {
        let read = stream.read_line(&mut result).await.unwrap();
        assert!(result.len() <= super::super::MAX_PROXY_HEADER_BYTES);
        assert!(read > 0);
        if result.ends_with("\r\n\r\n") {
            return result;
        }
    }
}
async fn closed(stream: &mut BufReader<TcpStream>) {
    let mut byte = [0; 1];
    match stream.read(&mut byte).await {
        Ok(count) => assert_eq!(count, 0),
        Err(error) => assert!(matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::UnexpectedEof
        )),
    }
}
#[tokio::test]
async fn accepted_http_streams_a_prefix_before_completion_and_strips_proxy_headers() {
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let fixture = Fixture::start().await;
        let mut client = BufReader::new(TcpStream::connect(fixture.address).await.unwrap());
        client.get_mut().write_all(b"GET http://fixture.invalid:8080/stream?part=1 HTTP/1.1\r\nHost: wrong.invalid\r\nConnection: x-hop, close\r\nX-Hop: canary\r\nProxy-Authorization: Basic canary\r\n\r\n").await.unwrap();
        let (backend,_) = fixture.upstream.accept().await.unwrap();
        let (release, hold) = oneshot::channel();
        let backend = tokio::spawn(async move {
            let mut backend = BufReader::new(backend);
            let request = headers(&mut backend).await.to_ascii_lowercase();
            assert!(request.starts_with("get /stream?part=1 http/1.1\r\n"));
            assert!(request.contains("\r\nhost: fixture.invalid:8080\r\n"));
            assert!(!request.contains("wrong.invalid"));
            assert!(!request.contains("x-hop"));
            assert!(!request.contains("proxy-authorization"));
            backend.get_mut().write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: x-hop, close\r\nX-Hop: canary\r\nProxy-Authenticate: Basic canary\r\n\r\nhello").await.unwrap();
            hold.await.unwrap();
            backend.get_mut().write_all(b" world").await.unwrap();
            backend.get_mut().shutdown().await.unwrap();
        });
        let response = headers(&mut client).await.to_ascii_lowercase();
        assert!(response.starts_with("http/1.1 200"));
        assert!(!response.contains("x-hop"));
        assert!(!response.contains("proxy-authenticate"));
        let mut prefix = [0; 5];
        let _read = client.read_exact(&mut prefix).await.unwrap();
        assert_eq!(&prefix,b"hello");
        assert!(!backend.is_finished());
        release.send(()).unwrap();
        let mut rest = [0; 6];
        let _read = client.read_exact(&mut rest).await.unwrap();
        assert_eq!(&rest,b" world");
        backend.await.unwrap();
        drop(client);
        fixture.stop().await;
    }).await.unwrap();
}
#[tokio::test]
async fn requested_websocket_upgrade_relays_frames_and_joined_close_ends_both_peers() {
    tokio::time::timeout(FIXTURE_DEADLINE,async {
        let fixture = Fixture::start().await;
        let mut client = BufReader::new(TcpStream::connect(fixture.address).await.unwrap());
        client.get_mut().write_all(b"GET http://fixture.invalid/socket HTTP/1.1\r\nHost: wrong.invalid\r\nConnection: upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nProxy-Authorization: Basic canary\r\n\r\n").await.unwrap();
        let (backend,_) = fixture.upstream.accept().await.unwrap();
        let backend = tokio::spawn(async move {
            let mut backend = BufReader::new(backend);
            let request = headers(&mut backend).await.to_ascii_lowercase();
            assert!(request.starts_with("get /socket http/1.1\r\n"));
            assert!(request.contains("\r\nhost: fixture.invalid\r\n"));
            assert!(request.contains("\r\nupgrade: websocket\r\n"));
            assert!(!request.contains("proxy-authorization"));
            backend.get_mut().write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\nProxy-Authenticate: Basic canary\r\n\r\n").await.unwrap();
            // One valid short masked client text frame for 'ping'.
            let mut frame = [0;10];
            let _read = backend.read_exact(&mut frame).await.unwrap();
            assert_eq!(&frame,b"\x81\x84\x01\x02\x03\x04qkmc");
            backend.get_mut().write_all(b"\x81\x04pong").await.unwrap();
            closed(&mut backend).await;
        });
        let response = headers(&mut client).await.to_ascii_lowercase();
        assert!(response.starts_with("http/1.1 101"));
        assert!(response.contains("\r\nupgrade: websocket\r\n"));
        assert!(!response.contains("proxy-authenticate"));
        client.get_mut().write_all(b"\x81\x84\x01\x02\x03\x04qkmc").await.unwrap();
        let mut response = [0;6];
        let _read = client.read_exact(&mut response).await.unwrap();
        assert_eq!(&response,b"\x81\x04pong");
        fixture.stop().await;
        closed(&mut client).await;
        backend.await.unwrap();
    }).await.unwrap();
}
#[tokio::test]
async fn accepted_connect_relays_opaque_bytes_and_joined_close_ends_both_peers() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let fixture = Fixture::start().await;
        let mut client = BufReader::new(TcpStream::connect(fixture.address).await.unwrap());
        client
            .get_mut()
            .write_all(b"CONNECT fixture.invalid:443 HTTP/1.1\r\nHost: wrong.invalid\r\n\r\n")
            .await
            .unwrap();
        let (backend, _) = fixture.upstream.accept().await.unwrap();
        let backend = tokio::spawn(async move {
            let mut backend = BufReader::new(backend);
            let mut request = [0; 5];
            let _read = backend.read_exact(&mut request).await.unwrap();
            assert_eq!(&request, b"probe");
            backend.get_mut().write_all(b"reply").await.unwrap();
            closed(&mut backend).await;
        });
        let response = headers(&mut client).await;
        assert!(response.starts_with("HTTP/1.1 200"));
        client.get_mut().write_all(b"probe").await.unwrap();
        let mut response = [0; 5];
        let _read = client.read_exact(&mut response).await.unwrap();
        assert_eq!(&response, b"reply");
        fixture.stop().await;
        closed(&mut client).await;
        backend.await.unwrap();
    })
    .await
    .unwrap();
}
