// What the session composer's box asks for and what its button does, in
// the references' words: agent-orchestrator's "Next message you send
// will be queued" while a turn runs, Paseo's "tag @files, or use
// /commands", its "Restore the session to continue" for an ended one.

import type { SendPlan } from "../../data/delivery";
import type { Delivery } from "../../wire/proto";
import type { SendAction } from "../compose/send-button";

export function composerPlaceholder({
  pending,
  live,
  again,
  plan,
  modern,
  starting = null,
  next = null,
}: {
  pending: boolean;
  live: boolean;
  /** Whether the ended session can be started again. */
  again: boolean;
  plan: SendPlan;
  /** Whether the machine speaks protocol 10 (`/` and `@`). */
  modern: boolean;
  /** The agent, while the session's first prompt waits on its start. */
  starting?: string | null;
  /** What a message would do while a turn runs, said before anything is
   * typed (the empty box's plan is Stop). */
  next?: Delivery | null;
}): string {
  if (pending) {
    return "Answer the permission to continue";
  }
  if (starting !== null && live && modern) {
    return `Queue a follow-up while ${starting} gets ready`;
  }
  if (!live) {
    return again
      ? "Start the session again to continue"
      : "The session has ended";
  }
  if (
    plan.kind === "stop" ||
    (plan.kind === "deliver" && plan.alternate !== null)
  ) {
    // A turn runs: say what Send will do with the next message.
    const delivery = plan.kind === "deliver" ? plan.delivery : next;
    return delivery === "steer"
      ? "Next message you send will steer the agent"
      : delivery === "interrupt"
        ? "Next message you send will stop the agent and go next"
        : modern
          ? "Next message you send will be queued"
          : "The agent is working";
  }
  return modern
    ? "Message the agent, tag @files, or use /commands"
    : "Message the agent";
}

export function composerAction({
  plan,
  editing,
  blocked,
  sending,
  agent,
  steerRefused,
  send,
  stop,
  update,
}: {
  plan: SendPlan;
  editing: boolean;
  blocked: boolean;
  sending: boolean;
  agent: string;
  steerRefused: boolean;
  send: (other: boolean) => void;
  stop: () => void;
  update: () => void;
}): SendAction {
  if (editing) {
    return {
      kind: "update",
      busy: sending,
      disabled: blocked,
      onUpdate: update,
    };
  }
  switch (plan.kind) {
    case "stop":
      return { kind: "stop", onStop: stop };
    case "idle":
      return { kind: "send", label: "Send", disabled: true, busy: sending };
    case "send":
      return { kind: "send", label: "Send", disabled: blocked, busy: sending };
    case "deliver":
      return {
        kind: "deliver",
        delivery: plan.delivery,
        alternate: plan.alternate,
        fallback:
          plan.fallback === null
            ? null
            : steerRefused
              ? `${agent} refused to steer: this queues`
              : `${agent} cannot steer: this queues`,
        turn: plan.alternate !== null,
        disabled: blocked,
        busy: sending,
        onSend: send,
      };
  }
}
