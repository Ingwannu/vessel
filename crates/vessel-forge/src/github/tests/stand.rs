//! The stand-in GitHub on loopback the provider's tests ask: canned
//! answers by `METHOD path`, a 304 for a matching `If-None-Match`, and
//! every request recorded with its token and `ETag`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use vessel_proto::forge::{ForgeKind, RepoRef};

use super::super::GitHub;
use crate::token::{Credentials, EnvTokens, Token};

pub(super) const TOKEN: &str = "ghp_loopback_test_token";
pub(super) const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

/// What the stand-in answers, by `METHOD path` (query included); and what
/// it was asked.
#[derive(Default)]
pub(super) struct Stand {
    pub(super) answers: HashMap<String, Canned>,
    pub(super) asked: Vec<(String, Option<String>, Option<String>)>,
    pub(super) graphql: Vec<GraphQlRequest>,
    pub(super) malformed_graphql: Option<serde_json::Error>,
}

/// The GraphQL wire is decoded once by this loopback edge, so tests
/// inspect typed variables instead of an untyped JSON tree.
#[derive(Debug, Deserialize)]
pub(super) struct GraphQlRequest {
    pub(super) query: String,
    pub(super) variables: GraphQlVariables,
}

#[derive(Debug, Deserialize)]
pub(super) struct GraphQlVariables {
    pub(super) owner: String,
    pub(super) name: String,
    pub(super) number: Option<vessel_proto::forge::PullNumber>,
    pub(super) first: Option<usize>,
    pub(super) after: Option<vessel_proto::forge::PullCursor>,
    pub(super) contexts: usize,
    pub(super) heads: usize,
}

pub(super) type Shared = Arc<Mutex<Stand>>;

/// One canned answer: its status, its headers and its body.
pub(super) type Canned = (StatusCode, Vec<(String, String)>, String);

async fn answer(
    State(stand): State<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let key = format!(
        "{method} {}",
        uri.path_and_query().map_or("", |path| path.as_str())
    );
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let mut stand = stand.lock().unwrap();
    stand.asked.push((
        key.clone(),
        header("authorization"),
        header("if-none-match"),
    ));
    if uri.path() == "/graphql" {
        match serde_json::from_slice(&body) {
            Ok(request) => stand.graphql.push(request),
            Err(error) => {
                stand.malformed_graphql = Some(error);
                return (StatusCode::BAD_REQUEST, "invalid fixture GraphQL request")
                    .into_response();
            }
        }
    }
    let Some((status, extra, body)) = stand.answers.get(&key).cloned() else {
        return (StatusCode::NOT_FOUND, format!("no answer for {key}")).into_response();
    };
    if status == StatusCode::OK
        && extra.iter().any(|(name, value)| {
            name == "etag" && header("if-none-match").as_deref() == Some(value.as_str())
        })
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    let mut response = (status, body).into_response();
    for (name, value) in extra {
        let _old = response.headers_mut().insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
    }
    response
}

pub(super) async fn stand() -> (Shared, GitHub, RepoRef) {
    let shared: Shared = Arc::new(Mutex::new(Stand::default()));
    let app = Router::new()
        .fallback(answer)
        .with_state(Arc::clone(&shared));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    }));
    let credentials = Arc::new(Credentials::new(
        EnvTokens {
            github: Token::new(TOKEN),
            enterprise: None,
        },
        None,
    ));
    let api = format!("http://{address}/").parse().unwrap();
    let github = GitHub::at(credentials, api).unwrap();
    let repo = RepoRef {
        kind: ForgeKind::Github,
        host: "github.com".parse().unwrap(),
        path: "owner/name".parse().unwrap(),
    };
    (shared, github, repo)
}

pub(super) fn set(
    shared: &Shared,
    key: &str,
    status: StatusCode,
    headers: &[(&str, &str)],
    body: &str,
) {
    let _old = shared.lock().unwrap().answers.insert(
        key.to_owned(),
        (
            status,
            headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body.to_owned(),
        ),
    );
}
