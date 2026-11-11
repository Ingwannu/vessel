// New session (agent-orchestrator's `/new`): "What do you want to work
// on?", one raised box for the prompt with what the session runs with in
// its foot, and where it runs in the tray under it. Send creates the
// session with those choices, sends the prompt, and opens the session.
// What each part decides is data/new-session.ts's.

import "./new-session.css";

import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useRef, useState } from "react";

import { useCrumbs, useFleet, useFleetState } from "../../context";
import {
  agentChoices,
  profileChoices,
  startingAgent,
} from "../../data/agent-choices";
import { agentName, runsTimeline } from "../../data/agents";
import { createSession, sendPrompt } from "../../data/calls";
import { folderSuggestions } from "../../data/folders";
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
import { toast } from "../../ui/toasts";
import type { HarnessKind } from "../../wire/proto";
import { AddMenu } from "./add-menu";
import { AgentMenu } from "./agent-menu";
import { PermissionsMenu } from "./permissions-menu";
import { PromptBox } from "./prompt-box";
import { SendButton } from "./send-button";
import { useAgentOptions } from "./use-agent-options";
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
};

const NO_CRUMBS: readonly never[] = [];

export function NewSession({ wanted }: { wanted: string | null }) {
  const fleet = useFleet();
  const navigate = useNavigate();
  const state = useFleetState((value) => value);
  const [draft, setDraft] = useState<Draft>(EMPTY_DRAFT);
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
  const agent: HarnessKind | null =
    harnesses.find((harness) => harness.kind === draft.agent)?.kind ??
    startingAgent(harnesses);
  const harness =
    harnesses.find((candidate) => candidate.kind === agent) ?? null;
  const effective: Draft = { ...draft, machine, agent };
  const terminal = harness !== null && startsTerminal(effective, harness);
  const options = useAgentOptions(machine, agent, draft.profile, !terminal);
  const controls =
    harness === null
      ? ({ kind: "terminal" } as const)
      : draftControls(effective, harness, options);
  const profiles =
    machine === null || agent === null
      ? []
      : profileChoices(state.profiles[machine] ?? [], agent);
  const sessions = machine === null ? undefined : state.sessions[machine];
  const folders = useMemo(() => folderSuggestions(sessions ?? []), [sessions]);
  const ready = sendable(effective, harness);

  const change = (next: Partial<Draft>) => {
    setError(null);
    setDraft((current) => ({ ...current, ...next }));
  };

  const send = () => {
    if (!ready.ok || harness === null || machine === null || busy) {
      return;
    }
    const spec = specOf(effective, harness, options);
    setBusy(true);
    setError(null);
    void createSession(fleet, machine, spec).then(async (created) => {
      if (!created.ok) {
        setBusy(false);
        setError(created.problem.message);
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
          value={draft.prompt}
          onChange={(prompt) => {
            change({ prompt });
          }}
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
                profile={draft.profile}
                onProfile={(profile) => {
                  change({ profile, picks: {} });
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
                      change({ agent: next, profile: "", picks: {} });
                    },
                  }}
                  state={controls}
                  onChoose={(choice) => {
                    change({ picks: pick(draft.picks, choice) });
                  }}
                />
              )}
              {controls.kind === "ready" &&
              controls.controls.permissions !== null ? (
                <PermissionsMenu
                  control={controls.controls.permissions}
                  onChoose={(choice) => {
                    change({ picks: pick(draft.picks, choice) });
                  }}
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
                change({ machine: next, agent: null, profile: "", picks: {} });
              }}
              folders={folders}
              folder={draft.folder}
              onFolder={(folder) => {
                change({ folder });
              }}
            />
          }
        />
        <div className="new-session-notes">
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
