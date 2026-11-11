// A running session's composer (GOAT's, on agent-orchestrator's box): the
// prompt, then the same agent/model/effort control and permissions
// control as the new-session page, reading the session's options from its
// log (`Timeline.options`) and changing them with `session.set_option`;
// then Send, or Stop while a turn runs. A pending permission is docked
// over its top edge, and the box waits until it is answered.
//
// A change shows when the node reports it (its `options_changed` record),
// not when it is asked for: what the page shows is what the agent runs
// with. While a change is on its way the controls wait; the agent's
// refusal is said in a toast.
//
// The text is the session's draft (kept by the bridge, emptied once the
// prompt is sent), and Up and Down in the empty box recall the session's
// own prompts (data/recall.ts).

import { useMemo, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { agentName } from "../../data/agents";
import { cancelTurn, sendPrompt, setOption } from "../../data/calls";
import { isLive } from "../../data/format";
import { reportedControls } from "../../data/options";
import { recall, sentPrompts, type Walk } from "../../data/recall";
import type { Timeline } from "../../data/timeline-model";
import { toast } from "../../ui/toasts";
import type { OptionChoice, SessionInfo } from "../../wire/proto";
import { AgentMenu } from "../compose/agent-menu";
import { PermissionsMenu } from "../compose/permissions-menu";
import { PromptBox } from "../compose/prompt-box";
import { SendButton } from "../compose/send-button";
import { useDraft } from "../compose/use-draft";
import { useShortcut } from "../shortcuts/handlers";
import { PermissionDock } from "./permission-dock";

export function Composer({
  node,
  session,
  timeline,
}: {
  node: string;
  session: SessionInfo;
  timeline: Timeline;
}) {
  const fleet = useFleet();
  const draft = useDraft(session.id);
  const text = draft.text;
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
  const [sending, setSending] = useState(false);
  const [changing, setChanging] = useState(false);
  const pending = timeline.pending;
  const live = isLive(session);
  const blocked = pending !== null || !live || sending;
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const harness = session.spec.harness;
  const controls = reportedControls(
    timeline.options,
    protocol,
    agentName(harness),
  );

  const send = () => {
    const prompt = text.trim();
    if (prompt === "" || blocked) {
      return;
    }
    setSending(true);
    void sendPrompt(fleet, node, session.id, prompt).then((outcome) => {
      setSending(false);
      if (outcome.ok) {
        walk.current = null;
        draft.clear();
      } else {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const choose = (choice: OptionChoice) => {
    setChanging(true);
    void setOption(fleet, node, session.id, choice).then((outcome) => {
      setChanging(false);
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const stop = () => {
    void cancelTurn(fleet, node, session.id).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const tool =
    pending === null
      ? undefined
      : timeline.entries.find(
          (entry) =>
            entry.kind === "tool" &&
            entry.tool.id === pending.request.tool_call,
        );
  return (
    <PromptBox
      label="Prompt"
      value={text}
      onChange={draft.setText}
      textAreaRef={area}
      note={
        draft.note === "too-long"
          ? "This draft is too long to keep; it stays in this tab until it is sent."
          : undefined
      }
      onArrow={(key, caret) => {
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
      placeholder={
        pending !== null
          ? "Answer the permission to continue"
          : live
            ? "Message the agent"
            : "The session has ended"
      }
      onSend={send}
      above={
        pending === null ? undefined : (
          <PermissionDock
            node={node}
            session={session.id}
            entry={pending}
            location={
              tool?.kind === "tool"
                ? (tool.tool.locations[0]?.path ?? null)
                : null
            }
          />
        )
      }
      controls={
        <>
          <AgentMenu
            agent={{
              value: harness,
              name: agentName(harness),
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
          action={
            timeline.inTurn && live
              ? { kind: "stop", onStop: stop }
              : {
                  kind: "send",
                  label: "Send",
                  disabled: blocked || text.trim() === "",
                  busy: sending,
                }
          }
        />
      }
    />
  );
}
