//! Streaming forwarding with one validated destination and owned cancellation.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, Response, StatusCode, header};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;

use super::Context;
use super::request::{self, Destination, RequestMode};

/// Backend handshake/response-header and upgrade waits are separately bounded.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) async fn serve(request: Request<Incoming>, context: Context) -> Response<Body> {
    let destination = match request::destination(&request) {
        Ok(destination) => destination,
        Err(_invalid) => return refused(StatusCode::BAD_REQUEST),
    };
    let stream = tokio::select! {
        () = context.cancel.cancelled() => return refused(StatusCode::SERVICE_UNAVAILABLE),
        stream = crate::network::dial_public(&destination.url, &context.preparation) => match stream {
            Ok(stream) => stream,
            Err(_refused) => return refused(StatusCode::FORBIDDEN),
        },
    };
    connected(request, destination, stream, context).await
}

// A private post-admission seam, never an alternate resolver or proxy entry point.
async fn connected(
    mut request: Request<Incoming>,
    destination: Destination,
    stream: tokio::net::TcpStream,
    context: Context,
) -> Response<Body> {
    if destination.mode == RequestMode::Connect {
        let upgrade = hyper::upgrade::on(&mut request);
        let task_context = context.clone();
        let _task = context.tasks.spawn(async move {
            let _admission = task_context.permit;
            tokio::select! {
                () = task_context.cancel.cancelled() => {},
                result = tokio::time::timeout(RESPONSE_TIMEOUT, upgrade) => {
                    if let Ok(Ok(upgraded)) = result {
                        let mut client = TokioIo::new(upgraded);
                        let mut stream = stream;
                        tokio::select! {
                            () = task_context.cancel.cancelled() => {},
                            _closed = tokio::io::copy_bidirectional(&mut client, &mut stream) => {},
                        }
                    }
                },
            }
        });
        return Response::new(Body::empty());
    }
    if request::prepare(&mut request, &destination).is_err() {
        return refused(StatusCode::BAD_REQUEST);
    }
    let cancel = context.cancel.clone();
    let result = tokio::select! {
        () = cancel.cancelled() => return refused(StatusCode::SERVICE_UNAVAILABLE),
        result = tokio::time::timeout(RESPONSE_TIMEOUT, forward(request, destination, stream, context)) => result,
    };
    match result {
        Ok(Ok(response)) => response,
        Ok(Err(_failed)) => refused(StatusCode::BAD_GATEWAY),
        Err(_deadline) => refused(StatusCode::GATEWAY_TIMEOUT),
    }
}

async fn forward(
    mut request: Request<Incoming>,
    destination: Destination,
    stream: tokio::net::TcpStream,
    context: Context,
) -> Result<Response<Body>, ForwardError> {
    let downstream = hyper::upgrade::on(&mut request);
    let (mut send, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .map_err(ForwardError::Http)?;
    let task_context = context.clone();
    let _task = context.tasks.spawn(async move {
        let _admission = task_context.permit;
        tokio::select! {
            () = task_context.cancel.cancelled() => {},
            _closed = connection.with_upgrades() => {},
        }
    });
    let mut response = send
        .send_request(request)
        .await
        .map_err(ForwardError::Http)?;
    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        if destination.mode != RequestMode::WebSocket
            || !response
                .headers()
                .get(header::UPGRADE)
                .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"websocket"))
        {
            return Err(ForwardError::Upgrade);
        }
        let upstream = hyper::upgrade::on(&mut response);
        let task_context = context.clone();
        let _task = context.tasks.spawn(async move {
            let _admission = task_context.permit;
            let pair = async { Ok::<_, hyper::Error>((downstream.await?, upstream.await?)) };
            tokio::select! {
                () = task_context.cancel.cancelled() => {},
                result = tokio::time::timeout(RESPONSE_TIMEOUT, pair) => {
                    if let Ok(Ok((downstream, upstream))) = result {
                        let mut client = TokioIo::new(downstream);
                        let mut server = TokioIo::new(upstream);
                        tokio::select! {
                            () = task_context.cancel.cancelled() => {},
                            _closed = tokio::io::copy_bidirectional(&mut client, &mut server) => {},
                        }
                    }
                },
            }
        });
        request::sanitize(response.headers_mut(), RequestMode::WebSocket)
            .map_err(ForwardError::Request)?;
    } else {
        request::sanitize(response.headers_mut(), RequestMode::Http)
            .map_err(ForwardError::Request)?;
    }
    Ok(response.map(Body::new))
}

#[derive(Debug, thiserror::Error)]
enum ForwardError {
    #[error("browser upstream HTTP operation failed")]
    Http(#[source] hyper::Error),
    #[error("browser upstream response headers are invalid")]
    Request(#[source] request::InvalidRequest),
    #[error("browser upstream upgrade was not a requested WebSocket")]
    Upgrade,
}

fn refused(status: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::from("Browser policy proxy refused this request"));
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests;
