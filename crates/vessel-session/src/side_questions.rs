//! A side question and its provider context are one actor command. No
//! sticky context can survive a busy refusal, cancellation or failed send.

use tokio::sync::oneshot;
use vessel_agent::PromptContext;
use vessel_proto::{Delivered, automation::PromptText};

use crate::SessionHandle;
use crate::error::SessionError;
use crate::moving::Control;
use crate::structured::StructuredActor;

#[derive(Debug)]
pub(crate) struct SidePrompt {
    pub(crate) question: PromptText,
    pub(crate) context: PromptContext,
    pub(crate) reply: oneshot::Sender<Result<Delivered, SessionError>>,
}

impl SessionHandle {
    /// Start a side turn, recording only the question. Context goes only
    /// into this provider request. Queue/steer are deliberately absent.
    ///
    /// # Errors
    /// The normal structured-input lifecycle and provider refusals.
    pub async fn side_question(
        &self,
        question: PromptText,
        context: PromptContext,
    ) -> Result<Delivered, SessionError> {
        let route = self.structured("a side question")?;
        self.order(route, |reply| {
            Control::SideQuestion(SidePrompt {
                question,
                context,
                reply,
            })
        })
        .await
    }
}

impl StructuredActor {
    pub(crate) async fn side_question(&mut self, order: SidePrompt) {
        let SidePrompt {
            question,
            context,
            reply,
        } = order;
        let result = self.start_side(question, context).await;
        let _gone = reply.send(result.map(|()| Delivered::Started));
    }

    async fn start_side(
        &mut self,
        question: PromptText,
        context: PromptContext,
    ) -> Result<(), SessionError> {
        // A parked side remains live: reuse the normal actor wake, whose
        // replay is suppressed, while retaining this command's context.
        // A failed wake consumes this command and cannot leave a prefix.
        if self.life.out.input_closed() {
            return Err(SessionError::ShuttingDown);
        }
        self.wake().await?;
        if self.life.out.input_closed() {
            return Err(SessionError::ShuttingDown);
        }
        let input = self.input.clone();
        self.pump(async move { input.prompt_context(question, context).await })
            .await
            .map_err(|_stopped| SessionError::Ended)??;
        self.desk.started_one();
        Ok(())
    }
}
