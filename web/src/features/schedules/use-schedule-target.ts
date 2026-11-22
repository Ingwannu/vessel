// The schedule asks the same machine catalogue as the new-session
// composer. Edits start with the stored spec so a late options probe or
// an unavailable project does not overwrite previously saved choices.
import { useState } from "react";

import { useAgents, useFleet, useFleetState } from "../../context";
import {
  agentChoices,
  profileChoices,
  type AgentChoice,
  type ProfileChoice,
  startingAgent,
} from "../../data/agent-choices";
import { agentName, runsTimeline, sameAgent } from "../../data/agents";
import type { OptionsState } from "../../data/new-session";
import { type Controls, controlsOf } from "../../data/options";
import {
  choicesFor,
  keptChoices,
  pick,
  type Picks,
  withPicks,
} from "../../data/picks";
import { type NewSessionSpec, sessionOptions } from "../../data/schedules/form";
import { useProjects } from "../../data/workspace/project-store";
import type { NodeKey } from "../../wire/bridge";
import type {
  Harness,
  HarnessRef,
  OptionChoice,
  ProfileInfo,
  ProjectId,
} from "../../wire/proto";
import { useAgentOptions } from "../compose/use-agent-options";

const NO_HARNESSES: readonly Harness[] = [];
const NO_PROFILES: readonly ProfileInfo[] = [];
export const KEEP_PLACE = "";

interface ScheduleTarget {
  reset: () => void;
  projectOptions: readonly { value: ProjectId; label: string; git: boolean }[];
  projectsLoading: boolean;
  project: ProjectId | null;
  chooseProject: (value: ProjectId | typeof KEEP_PLACE) => void;
  harness: HarnessRef | null;
  harnessLabel: string;
  agentOptions: readonly AgentChoice[];
  chooseAgent: (agent: HarnessRef) => void;
  profile: string;
  profileOptions: readonly ProfileChoice[];
  chooseProfile: (value: string) => void;
  chooseOption: (choice: OptionChoice) => void;
  originalModel: string | null;
  controls: Controls | null;
  options: OptionsState;
  again: () => void;
  spec: NewSessionSpec | null;
}

interface TargetDraft {
  project: ProjectId | null;
  agent: HarnessRef | null;
  profile: string | null;
  picks: Picks;
}

export function useScheduleTarget(
  node: NodeKey,
  original: NewSessionSpec | undefined,
): ScheduleTarget {
  const fleet = useFleet();
  const catalog = useFleetState(
    (state) => state.harnesses[node] ?? NO_HARNESSES,
  );
  const profiles = useFleetState(
    (state) => state.profiles[node] ?? NO_PROFILES,
  );
  const agents = useAgents(node);
  const projects = useProjects(fleet, node);
  const [draft, setDraft] = useState<TargetDraft>({
    project: null,
    agent: null,
    profile: null,
    picks: {},
  });
  const harnesses = catalog.filter(runsTimeline);
  const availableAgents = agentChoices(harnesses, agents);
  const harness = draft.agent ?? original?.harness ?? startingAgent(harnesses);
  const same = original !== undefined && sameAgent(harness, original.harness);
  const profile = draft.profile ?? (same ? (original.profile ?? "") : "");
  const keepOptions = same && profile === (original.profile ?? "");
  const stored = keepOptions ? (original.options ?? []) : [];
  const projectOptions =
    projects.kind === "ready"
      ? projects.list.projects
          .filter((project) => project.state.state === "ready")
          .map((project) => ({
            value: project.id,
            label: project.name,
            git: project.git,
          }))
      : [];
  const project = projectOptions.find((entry) => entry.value === draft.project);
  const withPlace = original !== undefined || project !== undefined;

  const { state: options, again } = useAgentOptions(
    node,
    harness,
    profile,
    withPlace,
  );
  const parts = sessionOptions(stored, Object.values(draft.picks));
  const optionPicks = parts.reduce<Picks>(pick, {});
  const controls =
    options.kind === "ready" && harness !== null
      ? controlsOf(
          withPicks(options.options, optionPicks),
          agentName(harness, agents),
        )
      : null;
  // A choice equal to today's default still has to replace an older
  // non-default option the saved definition holds.
  const optionChoices = keepOptions ? keptChoices : choicesFor;
  const changes =
    options.kind === "ready" ? optionChoices(options.options, draft.picks) : [];
  const combined = sessionOptions(stored, changes);
  const originalModel = keepOptions ? original.model : null;
  const model = controls?.model ?? null;
  const modelChanged = model !== null && draft.picks[model.id] !== undefined;
  const profileOptions =
    harness === null ? [] : profileChoices(profiles, harness);

  const spec: NewSessionSpec | null =
    !withPlace || harness === null
      ? null
      : {
          ...(original ?? {}),
          mode: "structured",
          harness,
          profile: profile === "" ? null : profile,
          ...(project === undefined
            ? {}
            : {
                cwd: null,
                place: { kind: "project", project: project.value },
              }),
          model: modelChanged ? null : (originalModel ?? null),
          options: combined,
        };

  return {
    reset: () => {
      setDraft({ project: null, agent: null, profile: null, picks: {} });
    },
    projectOptions,
    projectsLoading: projects.kind === "loading",
    project: project?.value ?? null,
    chooseProject: (value: ProjectId | typeof KEEP_PLACE) => {
      setDraft((current) => ({
        ...current,
        project: value === KEEP_PLACE ? null : value,
      }));
    },
    harness,
    harnessLabel: harness === null ? "" : agentName(harness, agents),
    agentOptions: availableAgents,
    chooseAgent: (agent: HarnessRef) => {
      if (!sameAgent(harness, agent)) {
        setDraft((current) => ({ ...current, agent, profile: "", picks: {} }));
      }
    },
    profile,
    profileOptions,
    chooseProfile: (value: string) => {
      setDraft((current) => ({ ...current, profile: value, picks: {} }));
    },
    chooseOption: (choice: OptionChoice) => {
      setDraft((current) => ({
        ...current,
        picks: pick(current.picks, choice),
      }));
    },
    originalModel: modelChanged ? null : (originalModel ?? null),
    controls,
    options,
    again,
    spec,
  };
}
