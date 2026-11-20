// Both launchers use the same composer so placement and editable agent
// options are validated consistently. A workspace already knows its place.
import "./new-session.css";

import { useNavigate } from "@tanstack/react-router";
import { useRef, useState } from "react";

import { useAgents, useCrumbs, useFleet, useFleetState } from "../../context";
import { chosenAgent, profileChoices } from "../../data/agent-choices";
import {
  agentModes,
  agentName,
  knownAgent,
  sameAgent,
} from "../../data/agents";
import {
  type Draft,
  draftControls,
  sendable,
  specOf,
  startsTerminal,
} from "../../data/new-session";
import { machineChoices } from "../../data/machines";
import { machineMeant } from "../../data/nodes";
import { pick } from "../../data/picks";
import { rememberedAgent, startingPicks } from "../../data/defaults";
import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { focusPage } from "../../ui/page-focus";
import type { OptionChoice } from "../../wire/proto";
import { trayOf } from "../../data/trays";
import { NewSessionControls } from "./new-session-controls";
import { NewSessionNotes } from "./new-session-notes";
import { AttachmentStrip } from "./attachment-strip";
import { PromptBox } from "./prompt-box";
import { SendButton } from "./send-button";
import {
  useNewSessionDraft,
  type WorkspaceDraft,
} from "./use-new-session-draft";
import { useDraftPresence } from "./use-draft-presence";
import { useShortcut } from "../shortcuts/handlers";
import { useAgentOptions } from "./use-agent-options";
import { useOptionsWait } from "./use-options-wait";
import { rememberStarted, sendFirst } from "./started";
import { useStarter } from "./use-starter";
import { WhereTray } from "./where-tray";

const EMPTY_DRAFT: Draft = {
  machine: null,
  agent: null,
  profile: "",
  terminal: false,
  args: "",
  folder: "",
  prompt: "",
  picks: {},
  chosen: false,
};
const NO_CRUMBS: readonly never[] = [];

export function NewSession({
  wanted,
  workspace,
}: {
  wanted: Draft["machine"];
  workspace?: WorkspaceDraft;
}) {
  const fleet = useFleet();
  const navigate = useNavigate();
  const state = useFleetState((value) => value);
  const [draft, setDraft] = useState<Draft>(EMPTY_DRAFT);
  const saved = useNewSessionDraft(workspace?.onDirtyChange);
  useShortcut("focus-composer", () => focusPage(text.current));
  const [busy, setBusy] = useState(false);
  // The starter hands these files over after the destination exists.
  const tray = trayOf(workspace?.tray ?? "new");
  const [error, setError] = useState<string | null>(null);
  const text = useRef<HTMLTextAreaElement>(null);
  useCrumbs(NO_CRUMBS);
  const here = useDraftPresence(text);

  const book = state.book ?? [];
  const machine =
    workspace === undefined
      ? (draft.machine ??
        machineMeant(
          book,
          [wanted, state.selected],
          (key) => state.statuses[key]?.state === "connected",
        ))
      : wanted;
  const listed = machine === null ? undefined : state.harnesses[machine];
  const harnesses = listed ?? [];
  const status = machine === null ? undefined : state.statuses[machine];
  const nodeId = status?.state === "connected" ? status.node.id : null;
  const defaults = useStore(prefs, (value) => value.values.defaults);
  const agents = useAgents(machine);
  const agent = chosenAgent(
    draft.agent,
    harnesses,
    rememberedAgent(defaults, nodeId, harnesses),
  );
  const known = agent === null ? null : knownAgent(agent);
  const harness =
    harnesses.find((candidate) => sameAgent(candidate.kind, agent)) ?? null;
  const modes = harness === null ? null : agentModes(harness);
  const agentLabel = agent === null ? "No agent" : agentName(agent, agents);
  const effective: Draft = {
    ...draft,
    machine,
    agent,
    prompt: saved.text,
    folder: workspace?.folder ?? draft.folder,
    ...startingPicks(draft, defaults, nodeId, known),
  };
  const terminal = harness !== null && startsTerminal(effective, harness);
  const { state: options, again } = useAgentOptions(
    machine,
    agent,
    effective.profile,
    !terminal,
  );
  const controls =
    harness === null
      ? ({ kind: "terminal" } as const)
      : draftControls(effective, harness, options, agentLabel);
  const profiles =
    machine === null || agent === null
      ? []
      : profileChoices(state.profiles[machine] ?? [], agent);
  const starter = useStarter(machine, workspace?.place);
  const ready = starter.ready(
    sendable(effective, harness, options, agentLabel, listed !== undefined),
  );

  const change = (next: Partial<Draft>) => {
    setError(null);
    setDraft((current) => ({ ...current, ...next }));
  };
  const restart = (next: Partial<Draft>) => {
    change({ ...next, profile: "", picks: {}, chosen: false });
  };
  const choose = (choice: OptionChoice) => {
    change({
      profile: effective.profile,
      picks: pick(effective.picks, choice),
      chosen: true,
    });
  };

  const start = () => {
    if (harness === null || machine === null) {
      setBusy(false);
      return;
    }
    const spec = specOf(effective, harness, options);
    void starter.create(spec).then((created) => {
      if (created?.ok !== true) {
        setBusy(false);
        setError(created?.problem.message ?? null);
        return;
      }
      const id = created.value.id;
      saved.clear();
      sendFirst(
        fleet,
        machine,
        id,
        spec,
        effective.prompt.trim(),
        workspace?.tray,
      );
      rememberStarted(fleet, nodeId, known, effective, options);
      fleet.select(machine);
      fleet.putSession(machine, created.value);
      void fleet.refreshSessions(machine);
      if (workspace !== undefined) {
        workspace.done(id);
      } else if (here.current) {
        void navigate({
          to: "/sessions/$node/$id",
          params: { node: machine, id },
          replace: true,
        });
      }
    });
  };
  // Sending early waits for the current agent options, just as /new does.
  const optionsWait = useOptionsWait(options.kind === "loading", start);
  const send = () => {
    if (!ready.ok || harness === null || machine === null || busy) {
      return;
    }
    setBusy(true);
    setError(null);
    if (!terminal && options.kind === "loading") {
      optionsWait.wait();
      return;
    }
    start();
  };

  return (
    <div className="new-session">
      <div className="new-session-column">
        <h1>What do you want to work on?</h1>
        <PromptBox
          label="Prompt"
          value={saved.text}
          onChange={(prompt) => {
            setError(null);
            saved.setText(prompt);
          }}
          note={
            workspace === undefined && saved.note === "too-long"
              ? "This draft is too long to keep; it stays in this tab until it is sent."
              : undefined
          }
          placeholder={
            terminal
              ? "A title for the terminal (optional)"
              : "Describe what the agent should do"
          }
          onSend={send}
          textAreaRef={text}
          {...(terminal
            ? {}
            : {
                onFiles: (files: File[]) => {
                  tray.add(files);
                },
                strip: <AttachmentStrip tray={tray} />,
              })}
          controls={
            <NewSessionControls
              profiles={profiles}
              draft={draft}
              effective={effective}
              change={change}
              restart={restart}
              terminal={terminal}
              modes={modes}
              harnesses={harnesses}
              agents={agents}
              agent={agent}
              agentLabel={agentLabel}
              controls={controls}
              choose={choose}
              tray={tray}
            />
          }
          action={
            <SendButton
              action={{
                kind: "send",
                label: terminal ? "Open the terminal" : "Send",
                disabled: !ready.ok,
                busy,
              }}
            />
          }
          tray={
            workspace === undefined ? (
              <WhereTray
                machines={machineChoices(book, state.statuses, true)}
                machine={machine}
                onMachine={(next) => {
                  restart({ machine: next, agent: null });
                }}
                starter={starter}
                folder={draft.folder}
                onFolder={(folder) => {
                  change({ folder });
                }}
              />
            ) : null
          }
        />
        <NewSessionNotes
          starting={
            optionsWait.waiting ? (
              <p className="new-session-note">
                Starting once {agentLabel} says what it offers…
              </p>
            ) : (
              starter.view
            )
          }
          error={error}
          blocked={ready.ok ? null : ready.reason}
          controls={controls}
          agent={agentLabel}
          onAskAgain={options.kind === "refused" ? again : null}
        />
      </div>
    </div>
  );
}
