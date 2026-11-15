//! Trusted node producers append plugin data through the existing session owner.
//! This is not an authorization or RPC surface: the broker must recheck its
//! actual target/session grant before obtaining the handle and invoking it.
use crate::moving::Control;
use crate::structured::StructuredActor;
use crate::{SessionError, SessionHandle};
use tokio::sync::oneshot;
use vessel_proto::timeline::PluginTimelineItem;
use vessel_proto::{AgentEvent, EventSeq, SessionEvent};

#[derive(Debug)]
pub(crate) struct PluginAppend {
    pub(crate) item: Box<PluginTimelineItem>,
    pub(crate) reply: oneshot::Sender<Result<EventSeq, SessionError>>,
}

impl SessionHandle {
    /// Append one already-validated plugin observation in actor order.
    /// The trusted node caller owns target authorization; this never starts a turn.
    /// The returned sequence is actor admission only. A durable producer must wait
    /// for the existing node persistence watermark before acknowledging its caller.
    /// # Errors
    /// A terminal, ended or moving session cannot accept this observation.
    pub async fn append_plugin(&self, item: PluginTimelineItem) -> Result<EventSeq, SessionError> {
        let route = self.structured("a plugin timeline observation")?;
        self.order(route, |reply| {
            Control::Plugin(PluginAppend {
                item: Box::new(item),
                reply,
            })
        })
        .await
    }
}

impl StructuredActor {
    pub(crate) async fn append_plugin(&mut self, order: PluginAppend) {
        let seq = self.life.out.next_seq();
        self.life
            .publish(
                &mut self.agent,
                SessionEvent::Agent {
                    event: AgentEvent::Plugin(*order.item),
                },
            )
            .await;
        let result = if self.life.out.next_seq() == seq {
            Err(SessionError::Ended)
        } else {
            Ok(seq)
        };
        let _closed = order.reply.send(result);
    }
}
