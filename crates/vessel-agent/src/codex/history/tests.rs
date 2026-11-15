//! Typed request/response tests exercise the actual transaction without a process.
use super::*;
use crate::HistoryTimestamp;
use crate::codex::wire::{Incoming, RpcError};
use serde_json::{Value, json};
const SAFETY_NET: Duration = Duration::from_secs(5);

async fn exchange(
    pages: Vec<Value>,
    cwd: Option<&Path>,
) -> Result<Option<HistoryPage>, ProbeError> {
    let (transaction, mut requests, responses) = Transaction::in_memory();
    let peer = async {
        let initialize: Value = serde_json::from_slice(&requests.recv().await.unwrap()).unwrap();
        assert_eq!(initialize["method"], "initialize");
        responses
            .send(Ok(Incoming::Response {
                id: 1,
                result: json!({"userAgent":"fake"}),
            }))
            .await
            .unwrap();
        let initialized: Value = serde_json::from_slice(&requests.recv().await.unwrap()).unwrap();
        assert_eq!(initialized["method"], "initialized");
        for (index, page) in pages.into_iter().enumerate() {
            let request: Value = serde_json::from_slice(&requests.recv().await.unwrap()).unwrap();
            assert_eq!(request["method"], "thread/list");
            assert_eq!(request["params"]["sortKey"], "updated_at");
            assert_eq!(
                request["params"]["cwd"],
                cwd.map(|p| p.to_string_lossy())
                    .map_or(Value::Null, |p| json!(p))
            );
            let id = u64::try_from(index).unwrap().checked_add(2).unwrap();
            responses
                .send(Ok(Incoming::Response { id, result: page }))
                .await
                .unwrap();
        }
    };
    tokio::time::timeout(SAFETY_NET, async {
        let (result, ()) = tokio::join!(read(transaction, cwd, HistoryFrom::default()), peer);
        result
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn actual_native_metadata_and_empty_filter_paging_are_preserved() {
    let page = exchange(vec![
        json!({"data":[{"id":"outside","cwd":"/elsewhere"}],"nextCursor":"page-two"}),
        json!({"data":[{"id":"native","cwd":"/work","name":"title","updatedAt":123},{"id":"unknown-time","cwd":"/work","preview":"preview"},{"id":"missing-folder"}]})
    ], Some(Path::new("/work"))).await.unwrap().unwrap();
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.entries.first().unwrap().session.as_str(), "native");
    assert_eq!(
        page.entries.first().unwrap().updated_at,
        Some(HistoryTimestamp::UnixSeconds(123))
    );
    assert_eq!(page.entries[1].updated_at, None);
    assert_eq!(page.entries[1].title.as_deref(), Some("preview"));
    assert_eq!(page.next, None);
}
#[tokio::test]
async fn cursor_cycles_and_invalid_native_ids_refuse_instead_of_empty_success() {
    assert!(matches!(
        exchange(
            vec![
                json!({"data":[],"nextCursor":"same"}),
                json!({"data":[{"id":"real","cwd":"/work"}],"nextCursor":"same"})
            ],
            None
        )
        .await,
        Err(ProbeError::Invalid)
    ));
    assert!(matches!(
        exchange(vec![json!({"data":[{"id":"","cwd":"/work"}]})], None).await,
        Err(ProbeError::Invalid)
    ));
}
#[tokio::test]
async fn only_method_not_found_is_unsupported_and_server_requests_are_refused() {
    for unsupported in [true, false] {
        let (mut transaction, mut requests, responses) = Transaction::in_memory();
        let peer = async {
            let _request = requests.recv().await.unwrap();
            let message = if unsupported {
                Incoming::Failure {
                    id: 1,
                    error: RpcError {
                        code: -32601,
                        message: "not found".into(),
                    },
                }
            } else {
                Incoming::Request {
                    id: json!("approval"),
                    method: "item/commandExecution/requestApproval".into(),
                    params: json!({}),
                }
            };
            responses.send(Ok(message)).await.unwrap();
        };
        let request = data::ListThreads {
            cursor: None,
            limit: MAX_ENTRIES,
            sort_key: data::SortKey::Updated,
            cwd: None,
        };
        let (result, ()) = tokio::time::timeout(SAFETY_NET, async {
            tokio::join!(transaction.request(&request), peer)
        })
        .await
        .unwrap();
        if unsupported {
            assert!(result.unwrap().is_none());
        } else {
            assert!(matches!(result, Err(ProbeError::Protocol)));
        }
        assert!(requests.try_recv().is_err());
    }
}
#[test]
fn missing_or_relative_directory_never_falls_back_to_process_directory() {
    let page: data::ThreadPage = serde_json::from_value(
        json!({"data":[{"id":"absent"},{"id":"relative","cwd":"relative"}]}),
    )
    .unwrap();
    assert!(page.entries(None, 0).unwrap().is_empty());
}
