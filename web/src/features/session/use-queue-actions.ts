// What the queue dock's buttons ask the node (`session.queue`): send a
// prompt now (a steer where the agent can, else stopping the turn; a
// turn of its own between turns), edit it in the composer, delete it,
// move it, and let a held queue move again. Each change shows when the
// node logs the queue after it; a refusal is said.

import { useFleet } from "../../context";
import { changeQueue, type QueueEdit } from "../../data/conversation-calls";
import { sendNowDelivery } from "../../data/delivery";
import type { QueueRow } from "../../data/queue-view";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { AgentAbilities, SessionId } from "../../wire/proto";
import type { QueueActions } from "./queue-dock";

export function useQueueActions({
  node,
  session,
  inTurn,
  abilities,
  steerRefused,
  onEdit,
}: {
  node: NodeKey;
  session: SessionId;
  inTurn: boolean;
  abilities: AgentAbilities | null;
  steerRefused: boolean;
  onEdit: (row: QueueRow) => void;
}): QueueActions {
  const fleet = useFleet();
  const change = (edit: QueueEdit) => {
    void changeQueue(fleet, node, session, edit).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const delivery = sendNowDelivery({ inTurn, abilities, steerRefused });
  return {
    sendNowLabel: delivery === "steer" ? "Steer" : "Send now",
    sendNowNote:
      delivery === "steer"
        ? "Steer into the running turn"
        : delivery === "interrupt"
          ? "Stop the turn and send this now"
          : "Start a turn with this message",
    onSendNow: (row) => {
      change({ change: "send_now", entry: row.id, delivery });
    },
    onEdit,
    onRemove: (row) => {
      change({ change: "remove", entry: row.id });
    },
    onMove: (row, before) => {
      change({ change: "move", entry: row.id, before });
    },
    onResume: () => {
      change({ change: "resume" });
    },
  };
}
