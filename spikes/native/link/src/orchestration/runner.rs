//! Typed mechanism seam allows fake nodes to exercise admission and deadlines.
use super::{Action, Output, Problem, Task};
use std::future::Future;
use std::time::Duration;
use vessel_proto::NodeId;

const DEADLINE: Duration = Duration::from_secs(15);

pub(super) trait Node: Sync {
    fn identity(&self) -> NodeId;
    fn supported(&self) -> bool;
    fn execute(&self, action: Action) -> impl Future<Output = Result<Output, Problem>> + Send;
}
pub(super) async fn run(node: &impl Node, task: &Task) -> Result<Output, Problem> {
    if node.identity() != task.scope.target.node() {
        return Err(Problem::Changed);
    }
    if !node.supported() {
        return Err(Problem::Unsupported);
    }
    tokio::time::timeout(DEADLINE, node.execute(task.action.clone()))
        .await
        .unwrap_or(Err(Problem::TimedOut))
}
