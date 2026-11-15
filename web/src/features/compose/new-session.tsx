// New session (agent-orchestrator's `/new`): "What do you want to work
// on?", one raised box for the prompt with what the session runs with in
// its foot, and where it runs in the tray under it. Send creates the
// session with those choices and opens it in place of this page as soon
// as it exists; the prompt goes without waiting on the agent's start
// (started.ts, data/openings.ts), as in agent-orchestrator and GOAT.
// What each part decides is data/new-session.ts's. Until the person
// chooses, the machine's last start fills in the agent, the profile and
// the choices (data/defaults.ts), and a successful start is kept as that.

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
import { useDraft } from "./use-draft";
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

export function NewSession({ wanted }: { wanted: Draft["machine"] }) {
  const fleet = useFleet();
  const navigate = useNavigate();
  const state = useFleetState((value) => value);
  const [draft, setDraft] = useState<Draft>(EMPTY_DRAFT);
  // The page's draft, kept by the bridge (back after a reload or launch).
  const saved = useDraft(null);
  useShortcut("focus-composer", () => focusPage(text.current));
  const [busy, setBusy] = useState(false);
  // Files for the first message, held until the session exists.
  const tray = trayOf("new");
  const [error, setError] = useState<string | null>(null);
  const text = useRef<HTMLTextAreaElement>(null);
  useCrumbs(NO_CRUMBS);
  const here = useDraftPresence(text);

  const book = state.book ?? [];
  const machine =
    draft.machine ??
    machineMeant(
      book,
      [wanted, state.selected],
      (key) => state.statuses[key]?.state === "connected",
    );
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
  // Kept per machine for the four only; an added agent starts on its own.
  const known = agent === null ? null : knownAgent(agent);
  const harness =
    harnesses.find((candidate) => sameAgent(candidate.kind, agent)) ?? null;
  const modes = harness === null ? null : agentModes(harness);
  const agentLabel = agent === null ? "No agent" : agentName(agent, agents);
  // What the machine's last start chose, while the person has chosen
  // nothing for this agent yet (data/defaults.ts).
  const effective: Draft = {
    ...draft,
    machine,
    agent,
    prompt: saved.text,
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
  const starter = useStarter(machine);
  const ready = starter.ready(
    sendable(effective, harness, options, agentLabel, listed !== undefined),
  );

  const change = (next: Partial<Draft>) => {
    setError(null);
    setDraft((current) => ({ ...current, ...next }));
  };
  // Another machine or agent: its choices start again from what it kept.
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
      sendFirst(fleet, machine, id, spec, effective.prompt.trim());
      rememberStarted(fleet, nodeId, known, effective, options);
      fleet.select(machine);
      void fleet.refreshSessions(machine);
      // agent-orchestrator's: in place of this page, and not for someone
      // who has left it meanwhile (the session is in the sidebar).
      if (here.current) {
        void navigate({
          to: "/sessions/$node/$id",
          params: { node: machine, id },
          replace: true,
        });
      }
    });
  };
  // Send during "Loading models…" waits for them (use-options-wait.ts).
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
            saved.note === "too-long"
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
