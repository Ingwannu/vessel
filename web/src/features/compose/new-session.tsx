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

import { useAgents, useCrumbs, useFleet, useFleetState } from "../../context";
import { chosenAgent, profileChoices } from "../../data/agent-choices";
import {
  agentModes,
  agentName,
  knownAgent,
  sameAgent,
} from "../../data/agents";
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
import type { OptionChoice } from "../../wire/proto";
import { handOver, sendWhenReady, trayOf } from "../../data/trays";
import { NewSessionControls } from "./new-session-controls";
import { NewSessionNotes } from "./new-session-notes";
import { AttachmentStrip } from "./attachment-strip";
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
  // Files for the first message, held until the session exists.
  const tray = trayOf("new");
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
  const ready = starter.ready(sendable(effective, harness, options));

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
      if (spec.mode === "structured" && tray.items.get().length > 0) {
        // Files go once the session's agent can take them: its composer
        // uploads them and sends this prompt (data/trays.ts).
        handOver("new", created.value.id);
        fleet.drafts.type(
          { kind: "session", session: created.value.id },
          prompt,
        );
        sendWhenReady(created.value.id);
      } else if (spec.mode === "structured" && prompt !== "") {
        const sent = await sendPrompt(fleet, machine, created.value.id, prompt);
        if (!sent.ok) {
          toast(
            `The session started; the prompt did not: ${sent.problem.message}`,
            "error",
          );
        }
      }
      saved.clear();
      if (nodeId !== null && known !== null) {
        const chosen = spec.mode === "structured" ? (spec.options ?? []) : [];
        for (const kept of rememberStart(
          nodeId,
          known,
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

  const agentLabel = agent === null ? "No agent" : agentName(agent, agents);
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
          starting={starter.view}
          error={error}
          blocked={ready.ok ? null : ready.reason}
          controls={controls}
          agent={agentLabel}
        />
      </div>
    </div>
  );
}
