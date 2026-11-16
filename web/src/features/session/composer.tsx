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

import { memo, useEffect, useMemo, useRef, useState } from "react";

import { useAgents, useFleetState } from "../../context";
import { agentName } from "../../data/agents";
import { hasConversation } from "../../data/delivery";
import { isLive } from "../../data/format";
import { canStartAgain } from "../../data/marks";
import { forgetOpening, openings } from "../../data/openings";
import { queueRows } from "../../data/queue-view";
import { recall, type Walk } from "../../data/recall";
import { useStore } from "../../data/store";
import { takeFirstSend, trayOf } from "../../data/trays";
import { AttachmentStrip } from "../compose/attachment-strip";
import { PromptBox } from "../compose/prompt-box";
import { useSessionRoot } from "../workspace/use-session-root";
import { SendButton } from "../compose/send-button";
import { useCaretAfterText } from "../compose/use-caret";
import { useDraft } from "../compose/use-draft";
import { useSuggest } from "../compose/use-suggest";
import { useShortcut } from "../shortcuts/handlers";
import { AgentControls } from "./agent-controls";
import { AttachButton } from "./attach-button";
import { ComposerDocks } from "./composer-docks";
import {
  type ComposerProps,
  promptsOf,
  sameComposerProps,
} from "./composer-props";
import { ContextRing } from "./context-ring";
import { EditingStrip } from "./editing-strip";
import { EndedBanner } from "./ended-banner";
import { composerAction, composerPlaceholder } from "./composer-words";
import { type Editing, useComposerSend } from "./use-composer-send";
import { useLimits } from "./use-limits";
import { useQueueActions } from "./use-queue-actions";
import { useTrayTarget } from "./use-tray-target";

/** Drawn again only when something it shows changed, not for every frame
 * of records (composer-props.ts says why and what is compared). */
export const Composer = memo(function Composer({
  node,
  label,
  session,
  timeline,
}: ComposerProps) {
  const draft = useDraft(session.id);
  const walk = useRef<Walk | null>(null);
  const area = useRef<HTMLTextAreaElement>(null);
  // A rewrite's caret is placed in the commit that shows its text, not a
  // frame later, where it undid a selection made meanwhile (use-caret.ts).
  const placeCaret = useCaretAfterText(area);
  useShortcut("focus-composer", () => {
    area.current?.focus();
    return area.current !== null;
  });
  const prompts = promptsOf(timeline.entries);
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
  const agent = agentName(harness, useAgents(node));
  const abilities = conversation.abilities ?? session.abilities ?? null;
  const tray = useMemo(() => trayOf(session.id), [session.id]);
  const limits = useLimits(node, session.id, modern && live);
  useTrayTarget(tray, node, session.id, live, abilities, limits, agent);
  const rows = useMemo(
    () => queueRows(conversation.queue),
    [conversation.queue],
  );
  // A first prompt from the new-session page on its way (data/openings.ts):
  // a message sent meanwhile is queued behind it (agent-orchestrator's
  // "Queue a follow-up while … gets ready"), and an empty box offers no
  // Stop; refused, it is back in the box, which says why until one goes.
  const opening = useStore(openings, (kept) => kept.get(session.id));
  const starting = opening?.state.kind === "sending";
  const refused =
    opening?.state.kind === "refused" ? opening.state.message : null;
  const sender = useComposerSend({
    node,
    session,
    timeline:
      starting && draft.text.trim() !== ""
        ? { ...timeline, inTurn: true }
        : timeline,
    protocol,
    draft: editing === null ? draft : { ...draft, text: editing.text },
    tray,
    editing,
    setEditing,
    rows,
    onSent: () => {
      walk.current = null;
      forgetOpening(session.id);
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
  // The code panel's root (one object while it names the same place):
  // a new one per drawing restarted `@`'s search after every answer.
  const sessionRoot = useSessionRoot(node, session).root;
  const suggest = useSuggest({
    node,
    commands: modern ? conversation.commands : [],
    root: sessionRoot,
    agent,
    text: area,
    apply: (next, caret) => {
      if (editing === null) {
        draft.setText(next);
      } else {
        setEditing({ ...editing, text: next });
      }
      placeCaret(next, caret);
    },
  });
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
        ) : refused !== null ? (
          <span className="prompt-problem" role="alert">
            The first message was not sent: {refused}
          </span>
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
        placeCaret(recalled.text, at);
        return true;
      }}
      disabled={blocked}
      placeholder={composerPlaceholder({
        pending: pending !== null,
        live,
        again: canStartAgain(session),
        plan: sender.plan,
        modern,
        starting: starting ? agent : null,
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
      strip={
        editing === null ? (
          <AttachmentStrip tray={tray} />
        ) : (
          <EditingStrip
            onCancel={() => {
              setEditing(null);
            }}
          />
        )
      }
      above={
        <>
          {!live && modern ? (
            <EndedBanner node={node} session={session} label={label} />
          ) : null}
          <ComposerDocks
            node={node}
            session={session.id}
            timeline={timeline}
            rows={rows}
            editing={editing?.row.id ?? null}
            actions={queueActions}
          />
        </>
      }
      controls={
        <>
          {modern && live ? <AttachButton tray={tray} limits={limits} /> : null}
          <ContextRing usage={conversation.usage} node={node} id={session.id} />
          <AgentControls
            node={node}
            session={session.id}
            harness={harness}
            agent={agent}
            options={timeline.options}
            protocol={protocol}
            live={live}
          />
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
}, sameComposerProps);
