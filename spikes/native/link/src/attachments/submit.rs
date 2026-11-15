//! Legacy prompt delivery retains its exact payload on an uncertain result.
//! This local Arc correlates ownership only; it never claims wire idempotence.
use super::{Answer, SubmitFailure, WithFiles};
use crate::{Event, State};
use std::{sync::Arc, time::Duration};
use vessel_proto::ContentBlock;

/// The link cannot hold command dispatch forever for a missing reply.
const DEADLINE: Duration = Duration::from_secs(30);

impl State {
    pub(crate) async fn prompt_files(&self, prompt: Arc<WithFiles>) {
        let result = async {
            let client = self.attachment_client(&prompt.machine, prompt.expected_node)?;
            let mut content = super::content(prompt.text.clone(), prompt.attachments.clone());
            if let Some(context) = &prompt.context {
                content.extend(context.attachments().map(|attachment| ContentBlock::Text {
                    text: attachment.text(),
                }));
            }
            tokio::time::timeout(
                DEADLINE,
                client.prompt(prompt.session, content, prompt.delivery),
            )
            .await
            .map_err(|_elapsed| SubmitFailure::TimedOut)?
            .map_err(SubmitFailure::Uncertain)
        }
        .await;
        self.emit(Event::Files(Answer::Submitted { prompt, result }))
            .await;
    }
}
