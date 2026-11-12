// A running session's composer (GOAT's, on agent-orchestrator's box): the
// prompt with its attached files over it, then the add button, the
// context meter and the agent's controls, and the one primary button.
// Over the box: the prompts waiting in the node's queue, then a pending
// permission or a question the agent asks; on its top edge the `/` and
// `@` list.
//
// A change shows when the node reports it (the log), not when it is
// asked for: an option, the queue, a question's end. The text is the
// session's draft (kept by the bridge), and Up and Down in the empty box
// recall the session's own prompts; a queued prompt opened for editing
// takes the box until it is saved or Escape closes it.

import { useEffect, useMemo, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { agentName } from "../../data/agents";
import { setOption } from "../../data/calls";
import { hasConversation } from "../../data/delivery";
import { isLive } from "../../data/format";
import { canStartAgain } from "../../data/marks";
import { reportedControls } from "../../data/options";
import { queueRows } from "../../data/queue-view";
import { searchRoot } from "../../data/suggest";
import { recall, sentPrompts, type Walk } from "../../data/recall";
import type { Timeline } from "../../data/timeline-model";
import { takeFirstSend, trayOf } from "../../data/trays";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { OptionChoice, SessionInfo } from "../../wire/proto";
import { AgentMenu } from "../compose/agent-menu";
import { AttachmentStrip } from "../compose/attachment-strip";
import { PermissionsMenu } from "../compose/permissions-menu";
import { PromptBox } from "../compose/prompt-box";
import { SendButton } from "../compose/send-button";
import { useDraft } from "../compose/use-draft";
import { useSuggest } from "../compose/use-suggest";
import { useShortcut } from "../shortcuts/handlers";
import { AttachButton } from "./attach-button";
import { ComposerDocks } from "./composer-docks";
import { ContextRing } from "./context-ring";
import { composerAction, composerPlaceholder } from "./composer-words";
import { type Editing, useComposerSend } from "./use-composer-send";
import { useLimits } from "./use-limits";
import { useQueueActions } from "./use-queue-actions";
import { useTrayTarget } from "./use-tray-target";

export function Composer({
  node,
  session,
  timeline,
}: {
  node: NodeKey;
  session: SessionInfo;
  timeline: Timeline;
}) {
  const fleet = useFleet();
  const draft = useDraft(session.id);
  const walk = useRef<Walk | null>(null);
  const area = useRef<HTMLTextAreaElement>(null);
  useShortcut("focus-composer", () => {
    area.current?.focus();
    return area.current !== null;
  });
  const prompts = useMemo(
    () => sentPrompts(timeline.entries),
    [timeline.entries],
  );
  const [changing, setChanging] = useState(false);
  const [editing, setEditing] = useState<Editing | null>(null);
  const conversation = timeline.conversation;
  const pending = timeline.pending;
  const live = isLive(session);
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const modern = hasConversation(protocol);
  const harness = session.spec.harness;
  const agent = agentName(harness);
  const abilities = conversation.abilities ?? session.abilities ?? null;
  const tray = useMemo(() => trayOf(session.id), [session.id]);
  const limits = useLimits(node, session.id, modern && live);
  useTrayTarget(tray, node, session.id, live, abilities, limits, agent);
  const rows = useMemo(
    () => queueRows(conversation.queue),
    [conversation.queue],
  );
  const sender = useComposerSend({
    node,
    session,
    timeline,
    protocol,
    draft: editing === null ? draft : { ...draft, text: editing.text },
    tray,
    editing,
    setEditing,
    rows,
    onSent: () => {
      walk.current = null;
    },
  });
  const queueActions = useQueueActions({
    node,
    session: session.id,
    inTurn: timeline.inTurn,
    abilities,
    steerRefused: sender.steerRefused,
    onEdit: (row) => {
      setEditing({ row, text: row.text });
      area.current?.focus();
    },
  });
  const text = editing === null ? draft.text : editing.text;
  const suggest = useSuggest({
    node,
    commands: modern ? conversation.commands : [],
    root: searchRoot(session.workspace),
    agent,
    text: area,
    apply: (next, caret) => {
      if (editing === null) {
        draft.setText(next);
      } else {
        setEditing({ ...editing, text: next });
      }
      window.requestAnimationFrame(() => {
        area.current?.setSelectionRange(caret, caret);
      });
    },
  });
  const controls = reportedControls(timeline.options, protocol, agent);
  const choose = (choice: OptionChoice) => {
    setChanging(true);
    void setOption(fleet, node, session.id, choice).then((outcome) => {
      setChanging(false);
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const blocked = pending !== null || !live || sender.sending;
  // A session the new-session page made with files: its first prompt is
  // sent from here once the agent has said what it takes and the machine
  // its limits (data/trays.ts). Taken once, on the first render.
  const firstSend = useRef<boolean | null>(null);
  firstSend.current ??= takeFirstSend(session.id);
  const canFirst =
    sender.plan.kind === "deliver" &&
    abilities !== null &&
    limits.kind === "read" &&
    !blocked;
  useEffect(() => {
    if (firstSend.current === true && canFirst) {
      firstSend.current = false;
      sender.send(false);
    }
  });
  return (
    <PromptBox
      label="Prompt"
      value={text}
      onChange={(next) => {
        sender.clearProblem();
        if (editing === null) {
          draft.setText(next);
        } else {
          setEditing({ ...editing, text: next });
        }
      }}
      textAreaRef={area}
      note={
        sender.problem !== null ? (
          <span className="prompt-problem" role="alert">
            {sender.problem}
          </span>
        ) : editing !== null ? (
          "Editing queued message · Escape to cancel"
        ) : draft.note === "too-long" ? (
          "This draft is too long to keep; it stays in this tab until it is sent."
        ) : undefined
      }
      onKey={(event) => {
        if (suggest.onKey(event)) {
          return true;
        }
        if (event.key === "Escape" && editing !== null) {
          setEditing(null);
          return true;
        }
        return false;
      }}
      onCaret={suggest.onCaret}
      menu={suggest.menu}
      onArrow={(key, caret) => {
        if (editing !== null) {
          return false;
        }
        const recalled = recall(key, caret, walk.current, prompts);
        if (recalled.kind === "pass") {
          return false;
        }
        walk.current = recalled.walk;
        draft.show(recalled.text);
        // Up leaves the caret at the start, so Up again goes older; Down
        // at the end (Paseo's edges).
        const at = key === "up" ? 0 : recalled.text.length;
        window.requestAnimationFrame(() => {
          area.current?.setSelectionRange(at, at);
        });
        return true;
      }}
      disabled={blocked}
      placeholder={composerPlaceholder({
        pending: pending !== null,
        live,
        again: canStartAgain(session),
        plan: sender.plan,
        modern,
      })}
      onSend={(other) => {
        if (editing === null) {
          sender.send(other);
        } else {
          sender.update();
        }
      }}
      {...(modern && live && limits.kind === "read"
        ? {
            onFiles: (files: File[]) => {
              tray.add(files);
            },
          }
        : {})}
      strip={<AttachmentStrip tray={tray} />}
      above={
        <ComposerDocks
          node={node}
          session={session.id}
          timeline={timeline}
          rows={rows}
          editing={editing?.row.id ?? null}
          actions={queueActions}
        />
      }
      controls={
        <>
          {modern && live ? <AttachButton tray={tray} limits={limits} /> : null}
          <ContextRing usage={conversation.usage} />
          <AgentMenu
            agent={{
              value: harness,
              name: agent,
              choices: null,
              onChange: () => undefined,
            }}
            state={controls}
            onChoose={choose}
            disabled={!live || changing}
          />
          {controls.kind === "ready" &&
          controls.controls.permissions !== null ? (
            <PermissionsMenu
              control={controls.controls.permissions}
              onChoose={choose}
              disabled={!live || changing}
            />
          ) : null}
        </>
      }
      action={
        <SendButton
          action={composerAction({
            plan: sender.plan,
            editing: editing !== null,
            blocked,
            sending: sender.sending,
            agent,
            steerRefused: sender.steerRefused,
            send: sender.send,
            stop: sender.stop,
            update: sender.update,
          })}
        />
      }
    />
  );
}
