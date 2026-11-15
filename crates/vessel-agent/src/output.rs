//! [`AgentOutput`] and [`AgentOutputs`]: what an agent session produces.

use tokio::sync::mpsc;
use vessel_proto::{AgentCommand, AgentEvent, ContextUsage, SessionOption};

use crate::error::Disconnect;
use crate::introduction::Introduction;

/// One output of an agent session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentOutput {
    /// A provider-owned, bounded plan proposal, distinct from a tool label.
    /// Only its owning session actor may offer correlated human decisions.
    PlanProposal(Box<vessel_proto::timeline::PlanProposal>),
    /// A timeline event. `AgentEvent::Ready` marks the end of the
    /// handshake; prompts are accepted from then on.
    Event(AgentEvent),
    /// The agent's config options are now these (protocol 9): once right
    /// before `AgentEvent::Ready`, as they stand after the handshake's
    /// choices (empty when the agent offers none), then whenever they
    /// change (the agent's own report, or a client's change). Not a
    /// timeline event: the session logs it as `options_changed`.
    Options(Vec<SessionOption>),
    /// The commands the agent offers now (protocol 10): the whole list,
    /// each time it differs from the last. The session logs it as
    /// `commands_changed`.
    Commands(Vec<AgentCommand>),
    /// How full the agent's context is (protocol 10), each time the
    /// agent's report differs from its last. Logged as `usage_changed`.
    Usage(ContextUsage),
    /// What the agent said of itself in `initialize` (protocol 11, ADR
    /// 0032): once, before the session's first `Options`. A session
    /// ignores it; a probe keeps it for `agent.status`.
    Introduced(Box<Introduction>),
    /// The ACP connection ended; the driver sends nothing after this.
    /// Harness log lines may still follow until the process's stderr ends.
    Disconnected(Disconnect),
}

/// The outputs of an agent session, in order.
///
/// Bounded: when the consumer stops reading, the driver stops, its
/// transport stops reading the agent's stdout, and the agent blocks on its
/// writes. Nothing is dropped. (What the SDK reads regardless is listed in
/// the `transport` module and ADR 0003.)
#[derive(Debug)]
pub struct AgentOutputs {
    receiver: mpsc::Receiver<AgentOutput>,
    details: crate::canonical::Snapshots,
}

impl AgentOutputs {
    pub(crate) fn new(receiver: mpsc::Receiver<AgentOutput>) -> Self {
        Self {
            receiver,
            details: crate::canonical::Snapshots::default(),
        }
    }

    /// The next output, or `None` once the driver and the stderr pump have
    /// both finished. Cancel-safe.
    pub async fn next(&mut self) -> Option<AgentOutput> {
        let mut output = self.receiver.recv().await?;
        if let AgentOutput::Event(event) = &mut output {
            self.details.observe(event);
        }
        Some(output)
    }
}
