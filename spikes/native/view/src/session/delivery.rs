//! What Send does with a prompt: Paseo's rules carried over (the owner's
//! word 45), from its composer (`app/src/composer/input/state.ts`
//! `resolveActiveSendBehavior`, `runDefaultSendAction`) and its server
//! (`agent/agent-prompt.ts`, `steerOrReplaceActiveTurn`).
//!
//! [Decision Log]
//! - Purpose: Settings' Default send (Queue, Steer, Interrupt) was read
//!   in the session area with Vessel's own rules: Queue by default, and a
//!   Steer the agent cannot take waited in the queue. Paseo defaults to
//!   Steer, replaces the turn when the agent cannot steer, and turns Queue
//!   into Interrupt while a permission waits.
//! - Constraint: the node takes a [`Delivery`] with each prompt; whether
//!   the agent can steer is its `abilities.steer`; a permission waiting is
//!   the conversation's (its log, or the node's word for a request the log
//!   no longer holds).
//! - Alternatives: keep the queue as the fallback for an agent that cannot
//!   steer (gentler, but not Paseo's: the person chose to reach the agent
//!   now); leave Queue alone behind a permission (Paseo's comment: a
//!   queued message would be stranded, the turn waits on the answer).
//! - Choice: Paseo's three rules in one function with its cases as tests,
//!   the app passing the facts.
//! - Cost: a Steer to an agent that cannot take one now stops its turn
//!   (Paseo's behaviour), where it used to wait; Settings' note says so.

use vessel_proto::Delivery;

use crate::prefs::TurnDelivery;

/// What the app knows when Send is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendFacts {
    /// A turn runs now.
    pub turn_running: bool,
    /// The agent takes a message into its running turn
    /// (`abilities.steer`).
    pub steers: bool,
    /// A permission request waits for the person's answer.
    pub permission_waits: bool,
}

/// How a plain send goes, given Settings' Default send (`chosen`).
///
/// With no turn running the prompt starts one (or waits behind prompts
/// already queued): [`Delivery::Queue`]. While one runs: Queue waits for
/// it to end, except behind a permission request, where it interrupts
/// (Paseo: the queued message would be stranded until the request is
/// answered); Steer joins the turn when the agent takes it and replaces
/// the turn otherwise (Paseo's server); Interrupt stops the turn and
/// starts the next.
#[must_use]
pub const fn send_delivery(chosen: TurnDelivery, facts: SendFacts) -> Delivery {
    if !facts.turn_running {
        return Delivery::Queue;
    }
    match chosen {
        TurnDelivery::Queue if facts.permission_waits => Delivery::Interrupt,
        TurnDelivery::Queue => Delivery::Queue,
        TurnDelivery::Steer if facts.steers => Delivery::Steer,
        TurnDelivery::Steer | TurnDelivery::Interrupt => Delivery::Interrupt,
    }
}

/// Whether a send going as `delivery` shows in the conversation at once
/// as the person's message (Paseo's pending message): every delivery but
/// one waiting in the queue, which shows in the queue's rows instead.
#[must_use]
pub const fn shows_at_once(delivery: Delivery, turn_running: bool) -> bool {
    !turn_running || !matches!(delivery, Delivery::Queue)
}
