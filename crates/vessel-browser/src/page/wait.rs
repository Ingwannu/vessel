//! Bounded condition polling shares the target's uncertain-delivery boundary.
use super::{Page, PageError, requests};
use serde::Deserialize;
use std::time::Duration;
use vessel_proto::browser::{WaitBudget, WaitFor, WaitMatch};

/// Reference service.ts:69 checks every25ms; this is not a test synchronization sleep.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Deserialize)]
struct Observed {
    value: bool,
}

impl Page {
    pub(crate) async fn wait(
        &mut self,
        condition: WaitFor,
        budget: WaitBudget,
    ) -> Result<WaitMatch, PageError> {
        self.active()?;
        let timeout = Duration::from_millis(u64::from(budget.host_timeout().get()));
        tokio::time::timeout(timeout, self.wait_condition(&condition))
            .await
            .map_err(PageError::WaitTimeout)?
    }

    async fn wait_condition(&mut self, condition: &WaitFor) -> Result<WaitMatch, PageError> {
        loop {
            if self.condition_matches(condition).await? {
                return Ok(match condition {
                    WaitFor::Text(_) => WaitMatch::Text,
                    WaitFor::Url(_) => WaitMatch::Url,
                });
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn condition_matches(&mut self, condition: &WaitFor) -> Result<bool, PageError> {
        let frame = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        let realm = self
            .execute(requests::IsolatedWorld::new(frame.id.clone()))
            .await?;
        let observed = self
            .execute(requests::ConditionProbe::new(
                realm.execution_context_id,
                condition,
            ))
            .await?
            .remote_object::<Observed>()
            .map_err(PageError::Evaluation)?;
        let after = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        if frame.id != after.id || frame.loader_id != after.loader_id {
            let _revision = self.invalidate()?;
            return Err(PageError::Navigation);
        }
        Ok(observed.value)
    }
}
