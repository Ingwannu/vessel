//! Sealed isolated-world commands, not an arbitrary execution-context selector.
use super::{Evaluated, Method, Request, sealed};
use crate::cdp::identity::{ContextId, FrameId, LoaderId};
use crate::snapshot::Config;
use serde::{Deserialize, Serialize};
const WORLD_NAME: &str = "vessel-browser-snapshot";

#[derive(Debug, Serialize)]
pub(crate) struct FrameTree {}
request!(FrameTree, FrameTreeResult, FrameTree);
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrameTreeResult {
    pub(crate) frame_tree: FrameRoot,
}
#[derive(Debug, Deserialize)]
pub(crate) struct FrameRoot {
    pub(crate) frame: Frame,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Frame {
    pub(crate) id: FrameId,
    pub(crate) loader_id: LoaderId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IsolatedWorld {
    frame_id: FrameId,
    world_name: &'static str,
    // The upstream wire spelling deliberately omits the second 's'.
    #[serde(rename = "grantUniveralAccess")]
    universal_access: bool,
}
impl IsolatedWorld {
    pub(crate) const fn new(frame_id: FrameId) -> Self {
        Self {
            frame_id,
            world_name: WORLD_NAME,
            universal_access: false,
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct World {
    pub(crate) execution_context_id: ContextId,
}
request!(IsolatedWorld, World, IsolatedWorld);

/// There is no public source/function/context constructor or generic CDP command tunnel.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SnapshotTraversal {
    function_declaration: &'static str,
    execution_context_id: ContextId,
    arguments: [Argument; 1],
    return_by_value: bool,
    await_promise: bool,
    user_gesture: bool,
}
#[derive(Debug, Serialize)]
struct Argument {
    value: Config,
}
impl SnapshotTraversal {
    pub(crate) const fn new(context: ContextId, config: Config) -> Self {
        Self {
            function_declaration: crate::snapshot::SOURCE,
            execution_context_id: context,
            arguments: [Argument { value: config }],
            return_by_value: true,
            await_promise: false,
            user_gesture: false,
        }
    }
}
impl std::fmt::Debug for SnapshotTraversal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedSnapshotTraversal([private])")
    }
}
request!(SnapshotTraversal, Evaluated, SnapshotTraversal);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_caller_selects_source_or_universal_access_and_context_zero_refuses() {
        let frame = FrameId::try_from("own-fixture-frame".to_owned()).unwrap();
        let encoded = serde_json::to_value(IsolatedWorld::new(frame)).unwrap();
        assert_eq!(encoded["grantUniveralAccess"], false);
        assert!(encoded.get("grantUniversalAccess").is_none());
        assert!(serde_json::from_str::<World>(r#"{"executionContextId":0}"#).is_err());
        assert!(serde_json::from_str::<World>(r#"{"executionContextId":-1}"#).is_err());
        let world: World = serde_json::from_str(r#"{"executionContextId":7}"#).unwrap();
        let mut refs = crate::references::References::<()>::default();
        let config = Config::new(refs.begin_snapshot().unwrap());
        let command = SnapshotTraversal::new(world.execution_context_id, config);
        assert!(!format!("{command:?}").contains("function"));
        let encoded = serde_json::to_value(command).unwrap();
        assert_eq!(encoded["functionDeclaration"], crate::snapshot::SOURCE);
        assert_eq!(encoded["arguments"][0]["value"]["firstRef"], "1");
        assert_eq!(
            encoded["arguments"][0]["value"]["limits"]["nodes"],
            crate::snapshot::MAX_SNAPSHOT_NODES
        );
        assert_eq!(encoded["userGesture"], false);
    }
}
