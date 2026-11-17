// What a running session's composer sends, and how (milestone 11, Track
// A-page): the plan for its one button (data/delivery.ts), its files
// uploaded first (data/tray.ts), the prompt with its delivery, Stop
// naming the turn it means, and a queued prompt edited in the box. What
// the node refused is kept to be said under the box, the text and the
// files kept with it.

import { useState } from "react";

import { useFleet } from "../../context";
import { cancelTurn, sendPrompt } from "../../data/calls";
import { changeQueue, promptSession } from "../../data/conversation-calls";
import {
  hasConversation,
  nextDelivery,
  sendPlan,
  type SendPlan,
  type SendState,
} from "../../data/delivery";
import { isLive } from "../../data/format";
import { seedImage } from "../../data/images";
import { errorCode } from "../../data/outcome";
import { prefs } from "../../data/prefs";
import type { QueueRow } from "../../data/queue-view";
import { withText } from "../../data/queue-view";
import { useStore } from "../../data/store";
import type { Timeline } from "../../data/timeline-model";
import type { Tray } from "../../data/tray";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { ContentBlock, Delivery, SessionInfo } from "../../wire/proto";
import type { DraftState } from "../compose/use-draft";

/** A queued prompt open in the composer. */
export interface Editing {
  row: QueueRow;
  text: string;
}

export interface ComposerSend {
  plan: SendPlan;
  /** What a message would do while a turn runs (`nextDelivery`). */
  next: Delivery | null;
  sending: boolean;
  /** What the node or the tray refused, to say under the box. */
  problem: string | null;
  steerRefused: boolean;
  send: (other: boolean) => void;
  stop: () => void;
  /** Save the open edit (`editing`) to the node. */
  update: () => void;
  /** Forget what was refused (the text changed). */
  clearProblem: () => void;
}

export function useComposerSend({
  node,
  session,
  timeline,
  protocol,
  draft,
  tray,
  editing,
  setEditing,
  rows,
  onSent,
}: {
  node: NodeKey;
  session: SessionInfo;
  timeline: Timeline;
  protocol: number | null;
  draft: DraftState;
  tray: Tray;
  editing: Editing | null;
  setEditing: (editing: Editing | null) => void;
  /** The queue as the node last said, for an edit's revision. */
  rows: readonly QueueRow[];
  onSent: () => void;
}): ComposerSend {
  const fleet = useFleet();
  const preferred = useStore(prefs, (state) => state.values.composer.delivery);
  const [sending, setSending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [steerRefused, setSteerRefused] = useState(false);
  const filled = useStore(tray.items, (items) => items.length > 0);
  const text = draft.text;
  // A waiting permission blocks only where nothing can be delivered
  // during a turn (before protocol 10); from 10 the rules decide (794).
  const blocked =
    (timeline.pending !== null && !hasConversation(protocol)) || sending;
  const state: SendState = {
    protocol,
    live: isLive(session),
    inTurn: timeline.inTurn,
    filled: text.trim() !== "" || filled,
    abilities: timeline.conversation.abilities ?? session.abilities ?? null,
    queue: timeline.conversation.queue,
    preferred,
    steerRefused,
    permissionWaits: timeline.pending !== null,
  };
  const plan = sendPlan(state);

  const send = (other: boolean) => {
    if (blocked) {
      return;
    }
    switch (plan.kind) {
      case "idle":
      case "stop":
        return;
      case "send": {
        const prompt = text.trim();
        setSending(true);
        void sendPrompt(fleet, node, session.id, prompt).then((outcome) => {
          setSending(false);
          if (outcome.ok) {
            draft.clear();
            onSent();
          } else {
            setProblem(outcome.problem.message);
          }
        });
        return;
      }
      case "deliver":
        break;
    }
    const delivery =
      other && plan.alternate !== null ? plan.alternate : plan.delivery;
    setSending(true);
    setProblem(null);
    void (async () => {
      const ready = await tray.upload();
      if (ready.kind !== "ready") {
        setSending(false);
        setProblem(
          ready.kind === "blocked" ? ready.reason : "A file is still uploading",
        );
        return;
      }
      const words = text.trim();
      const content: ContentBlock[] = [
        ...(words === "" ? [] : [{ type: "text" as const, text: words }]),
        ...ready.refs.map((attachment) => ({
          type: "attachment" as const,
          attachment,
        })),
      ];
      const outcome = await promptSession(
        fleet,
        node,
        session.id,
        content,
        delivery,
      );
      setSending(false);
      if (outcome.ok) {
        draft.clear();
        tray.sent((ref, image) => {
          seedImage(node, session.id, ref.id, image);
        });
        onSent();
        return;
      }
      if (
        delivery === "steer" &&
        errorCode(outcome.problem) === "unsupported"
      ) {
        setSteerRefused(true);
      }
      setProblem(outcome.problem.message);
    })();
  };

  const update = () => {
    if (editing === null || sending) {
      return;
    }
    const current = rows.find((row) => row.id === editing.row.id);
    if (current === undefined) {
      toast("That message was already sent or removed.");
      draft.setText(editing.text);
      setEditing(null);
      return;
    }
    setSending(true);
    const files: ContentBlock[] = current.attachments.map((attachment) => ({
      type: "attachment",
      attachment,
    }));
    void changeQueue(fleet, node, session.id, {
      change: "edit",
      entry: current.id,
      revision: current.revision,
      content: withText(files, editing.text.trim()),
    }).then((outcome) => {
      setSending(false);
      if (outcome.ok) {
        setEditing(null);
        return;
      }
      const code = errorCode(outcome.problem);
      if (code === "conflict") {
        toast(
          "This message was changed elsewhere. Update again to replace it with yours.",
        );
      } else if (code === "not_found") {
        toast("That message was already sent or removed.");
        draft.setText(editing.text);
        setEditing(null);
      } else {
        toast(outcome.problem.message, "error");
      }
    });
  };

  const stop = () => {
    const turn = hasConversation(protocol) ? timeline.turn : null;
    void cancelTurn(fleet, node, session.id, turn).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };

  const clearProblem = () => {
    if (problem !== null) {
      setProblem(null);
    }
  };
  return {
    plan,
    next: nextDelivery(state),
    sending,
    problem,
    steerRefused,
    send,
    stop,
    update,
    clearProblem,
  };
}
