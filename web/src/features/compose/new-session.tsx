// New session (agent-orchestrator's `/new`): "What do you want to work
// on?", one raised box for the prompt with what the session runs with in
// its foot, and where it runs in the tray under it. Send creates the
// session with those choices, sends the prompt, and opens the session.
// What each part decides is data/new-session.ts's. Until the person
// chooses, the machine's last start fills in the agent, the profile and
// the choices (data/defaults.ts), and a successful start is kept as that.

import "./new-session.css";

import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

import { useCrumbs, useFleet, useFleetState } from "../../context";
import {
  agentChoices,
  profileChoices,
  startingAgent,
} from "../../data/agent-choices";
import { agentName, knownAgent, runsTimeline } from "../../data/agents";
import { sendPrompt } from "../../data/calls";
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
import {
  rememberedAgent,
  startingPicks,
  rememberStart,
} from "../../data/defaults";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { toast } from "../../ui/toasts";
import type { HarnessKind, OptionChoice } from "../../wire/proto";
import { AddMenu } from "./add-menu";
import { AgentMenu } from "./agent-menu";
import { PermissionsMenu } from "./permissions-menu";
import { PromptBox } from "./prompt-box";
import { SendButton } from "./send-button";
import { useDraft } from "./use-draft";
import { useShortcut } from "../shortcuts/handlers";
import { useAgentOptions } from "./use-agent-options";
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
  // The prompt is the new-session page's draft, kept by the bridge: it
  // comes back after leaving the page, a reload or a new launch.
  const saved = useDraft(null);
  useShortcut("focus-composer", () => {
    text.current?.focus();
    return text.current !== null;
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const text = useRef<HTMLTextAreaElement>(null);
  useCrumbs(NO_CRUMBS);
  useEffect(() => {
    text.current?.focus();
  }, []);

  const book = state.book ?? [];
  const machine =
    draft.machine ??
    machineMeant(
      book,
      [wanted, state.selected],
      (key) => state.statuses[key]?.state === "connected",
    );
  const harnesses = machine === null ? [] : (state.harnesses[machine] ?? []);
  const status = machine === null ? undefined : state.statuses[machine];
  const nodeId = status?.state === "connected" ? status.node.id : null;
  const defaults = useStore(prefs, (value) => value.values.defaults);
  const chosen = harnesses.find((harness) => harness.kind === draft.agent);
  const agent: HarnessKind | null =
    (chosen === undefined ? null : knownAgent(chosen.kind)) ??
    rememberedAgent(defaults, nodeId, harnesses) ??
    startingAgent(harnesses);
  const harness =
    harnesses.find((candidate) => candidate.kind === agent) ?? null;
  // What the machine's last start chose, while the person has chosen
  // nothing for this agent yet (data/defaults.ts).
  const effective: Draft = {
    ...draft,
    machine,
    agent,
    prompt: saved.text,
    ...startingPicks(draft, defaults, nodeId, agent),
  };
  const terminal = harness !== null && startsTerminal(effective, harness);
  const options = useAgentOptions(machine, agent, effective.profile, !terminal);
  const controls =
    harness === null
      ? ({ kind: "terminal" } as const)
      : draftControls(effective, harness, options);
  const profiles =
    machine === null || agent === null
      ? []
      : profileChoices(state.profiles[machine] ?? [], agent);
  const starter = useStarter(machine);
  const ready = starter.ready(sendable(effective, harness));

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

  const send = () => {
    if (!ready.ok || harness === null || machine === null || busy) {
      return;
    }
    const spec = specOf(effective, harness, options);
    setBusy(true);
    setError(null);
    void starter.create(spec).then(async (created) => {
      if (created?.ok !== true) {
        setBusy(false);
        setError(created?.problem.message ?? null);
        return;
      }
      const prompt = effective.prompt.trim();
      if (spec.mode === "structured" && prompt !== "") {
        const sent = await sendPrompt(fleet, machine, created.value.id, prompt);
        if (!sent.ok) {
          toast(
            `The session started; the prompt did not: ${sent.problem.message}`,
            "error",
          );
        }
      }
      saved.clear();
      if (nodeId !== null && agent !== null) {
        const chosen = spec.mode === "structured" ? (spec.options ?? []) : [];
        for (const kept of rememberStart(
          nodeId,
          agent,
          effective.profile,
          chosen,
        )) {
          void prefer(fleet, kept);
        }
      }
      fleet.select(machine);
      void fleet.refreshSessions(machine);
      void navigate({
        to: "/sessions/$node/$id",
        params: { node: machine, id: created.value.id },
      });
    });
  };

  const agentLabel = agent === null ? "No agent" : agentName(agent);
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
          controls={
            <>
              <AddMenu
                profiles={profiles}
                profile={effective.profile}
                onProfile={(profile) => {
                  change({ profile, picks: {}, chosen: true });
                }}
                terminal={draft.terminal}
                canTimeline={harness === null || runsTimeline(harness)}
                onTerminal={(on) => {
                  change({ terminal: on });
                }}
                args={draft.args}
                onArgs={(args) => {
                  change({ args });
                }}
              />
              {agent === null ? null : (
                <AgentMenu
                  agent={{
                    value: agent,
                    name: agentLabel,
                    choices: agentChoices(harnesses),
                    onChange: (next) => {
                      restart({ agent: next });
                    },
                  }}
                  state={controls}
                  onChoose={choose}
                />
              )}
              {controls.kind === "ready" &&
              controls.controls.permissions !== null ? (
                <PermissionsMenu
                  control={controls.controls.permissions}
                  onChoose={choose}
                />
              ) : null}
            </>
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
        <div className="new-session-notes">
          {starter.view}
          {error === null ? null : (
            <p className="new-session-error" role="alert">
              {error}
            </p>
          )}
          {!ready.ok && ready.reason !== null ? (
            <p className="new-session-note">{ready.reason}</p>
          ) : null}
          {controls.kind === "failed" ? (
            <p className="new-session-note" role="status">
              {agentLabel} did not say what it offers: {controls.message} It
              starts on its own defaults.
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
