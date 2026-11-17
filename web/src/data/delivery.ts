// What the composer's one button does (milestone 11, Track A-page):
// between turns it sends; while a turn runs it stops the turn when the
// box is empty, and with text it delivers the way the person chose in
// Settings › General (queue, steer, interrupt), the other way one
// modifier away (⌘/Ctrl with the click or with Enter).
//
// [Decision Log]
// - Purpose: protocol 10 lets a prompt typed during a turn wait in the
//   node's queue, join the turn (steering, where the agent can), or stop
//   the turn and go next. The page must offer exactly what the agent and
//   the node do, and nothing that would be refused.
// - Constraint: steering is the agent's to say (`abilities.steer`, from
//   its handshake); queue and interrupt are the node's and work for every
//   agent. A node before protocol 10 has none of the three: the page then
//   keeps wave 1's Send and Stop.
// - Alternatives: a menu of deliveries beside the button (none of the
//   references has one); one button per delivery (three round buttons
//   in a 28px row).
// - Choice: the references' one button whose icon says what it will do
//   (agent-orchestrator's arrow to queue, GOAT's and T3 Code's turn for a
//   steer), the default from the preference (Paseo's "Default send"),
//   the other way with the modifier, as all three do. The other way of
//   queue is steer where the agent can and interrupt where it cannot; of
//   steer and interrupt, queue (Paseo's pairs). An agent that cannot
//   steer queues instead and the button says so; the person's choice is
//   kept for the next agent that can.
// - Between turns a held queue (the person stopped a turn) does not take
//   a new message: it starts now, and what was held stays held until the
//   person resumes it (agent-orchestrator's "an empty queue is never
//   held", GOAT's "nothing waiting is sent after a stop").

import type { TurnDelivery } from "../wire/bridge";
import type { AgentAbilities, Delivery, PromptQueue } from "../wire/proto";

/** The first protocol with the conversation's calls. */
export const CONVERSATION_PROTOCOL = 10;

/** Whether a connection speaking `protocol` has protocol 10's
 * conversation. */
export function hasConversation(protocol: number | null): boolean {
  return protocol !== null && protocol >= CONVERSATION_PROTOCOL;
}

/** What the page knows when it decides. */
export interface SendState {
  /** The connection's protocol; `null` while not connected. */
  protocol: number | null;
  /** Whether the session is live. */
  live: boolean;
  /** Whether a turn runs. */
  inTurn: boolean;
  /** Whether the box holds something to send. */
  filled: boolean;
  /** What the agent said it can do; `null` before it said. */
  abilities: AgentAbilities | null;
  /** The node's queue, as its last `queue_changed` left it. */
  queue: PromptQueue | null;
  /** The person's choice for a message sent during a turn. */
  preferred: TurnDelivery;
  /** Whether the node refused a steer for this session ("method not
   * found" from an agent that said it could). */
  steerRefused: boolean;
}

/** What the button does. */
export type SendPlan =
  /** Nothing to do: ended, or empty between turns. */
  | { kind: "idle" }
  /** Stop the running turn (the box is empty). */
  | { kind: "stop" }
  /** Send as protocol 2 does (`session.send`): a node before 10. */
  | { kind: "send" }
  /** Send with a delivery; `alternate` is what the modifier sends
   * instead. `fallback` says why the preferred delivery is not the one
   * used. */
  | {
      kind: "deliver";
      delivery: Delivery;
      alternate: Delivery | null;
      fallback: "cannot-steer" | null;
    };

/** Whether the agent can take a message into a running turn. */
export function canSteer(state: SendState): boolean {
  return state.abilities?.steer === true && !state.steerRefused;
}

/** What the composer's button does in `state`. */
export function sendPlan(state: SendState): SendPlan {
  if (!state.live) {
    return { kind: "idle" };
  }
  if (!hasConversation(state.protocol)) {
    if (state.inTurn) {
      return { kind: "stop" };
    }
    return state.filled ? { kind: "send" } : { kind: "idle" };
  }
  if (!state.filled) {
    return state.inTurn ? { kind: "stop" } : { kind: "idle" };
  }
  if (!state.inTurn) {
    return {
      kind: "deliver",
      delivery: betweenTurns(state.queue),
      alternate: null,
      fallback: null,
    };
  }
  const steers = canSteer(state);
  const wanted: Delivery = state.preferred;
  const delivery = wanted === "steer" && !steers ? "queue" : wanted;
  return {
    kind: "deliver",
    delivery,
    alternate:
      delivery === "queue" ? (steers ? "steer" : "interrupt") : "queue",
    fallback: delivery === wanted ? null : "cannot-steer",
  };
}

/** The delivery a message would take while a turn runs, whatever the box
 * holds now, so the empty box can say what Enter will do; `null` between
 * turns and on a node before protocol 10. */
export function nextDelivery(state: SendState): Delivery | null {
  if (!state.inTurn) return null;
  const plan = sendPlan({ ...state, filled: true });
  return plan.kind === "deliver" ? plan.delivery : null;
}

/** Between turns: `queue` keeps the order of what already waits and
 * starts at once when nothing does; a held queue keeps what it holds and
 * the new message starts now. */
function betweenTurns(queue: PromptQueue | null): Delivery {
  const held = queue?.held;
  return held === undefined || held === null ? "queue" : "start";
}

/** A delivery in the button's words (its aria label and tooltip),
 * agent-orchestrator's and GOAT's: "Queue message", "Steer message". */
export function deliveryLabel(delivery: Delivery): string {
  switch (delivery) {
    case "start":
      return "Send";
    case "queue":
      return "Queue message";
    case "steer":
      return "Steer message";
    case "interrupt":
      return "Interrupt and send";
  }
}

/** What the tooltip adds after the label (agent-orchestrator's "Steer
 * into running turn"). */
export function deliveryNote(delivery: Delivery): string {
  switch (delivery) {
    case "start":
      return "Starts a turn now";
    case "queue":
      return "Sends when the agent finishes its turn";
    case "steer":
      return "Steer into the running turn";
    case "interrupt":
      return "Stops the turn and sends this now";
  }
}

/** The delivery a queued prompt's "Send now" uses: start between turns;
 * during one, steer where the agent can, else interrupt (`queue` is not a
 * delivery there: the node refuses it). */
export function sendNowDelivery(
  state: Pick<SendState, "inTurn" | "abilities" | "steerRefused">,
): "start" | "steer" | "interrupt" {
  if (!state.inTurn) {
    return "start";
  }
  return state.abilities?.steer === true && !state.steerRefused
    ? "steer"
    : "interrupt";
}
