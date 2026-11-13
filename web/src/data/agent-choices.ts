// What the new-session page chooses among a machine's agents (track W1):
// the agents to pick from (the four and, from protocol 11, the agents
// added on the machine), the one it starts with, what it says when the
// machine will not start one, an agent's profiles, and where a running
// session may move among them. The agent's name and kinds are
// `agents.ts`'s.

import type {
  AgentInfo,
  Harness,
  HarnessKind,
  HarnessRef,
  ProfileInfo,
} from "../wire/proto";
import {
  addedAgents,
  agentKey,
  agentModes,
  agentName,
  knownAgent,
  runsTimeline,
  sameAgent,
} from "./agents";

/** One agent to choose, in the kit's choice shape: `value` is the agent's
 * key (`agentKey`), which a menu compares; `agent` is the agent. */
export interface AgentChoice {
  value: string;
  agent: HarnessRef;
  label: string;
  detail?: string;
}

/** A machine's agents to choose from: the four it found, each with its
 * version as the detail, then the agents added on it whose program it
 * found (ADR 0032), by their label (`agents`, its `agent.list`). An added
 * agent the machine cannot run either way is not offered. */
export function agentChoices(
  harnesses: readonly Harness[],
  agents: readonly AgentInfo[] = [],
): AgentChoice[] {
  const known = knownOf(harnesses).map(({ harness, kind }) => {
    const version = harness.version?.split(/\s+/)[0] ?? null;
    return {
      value: agentKey(kind),
      agent: kind,
      label: agentName(kind),
      ...(version === null || version === "" ? {} : { detail: version }),
    };
  });
  const added = harnesses
    .filter(
      (harness) =>
        knownAgent(harness.kind) === null &&
        agentModes(harness).kind !== "neither",
    )
    .map((harness) => ({
      value: agentKey(harness.kind),
      agent: harness.kind,
      label: agentName(harness.kind, agents),
      detail: "added",
    }));
  return [...known, ...added];
}

/** The agent a new session starts with: the first of the four that runs
 * with a timeline, else the first of them installed, else the first
 * agent added on the machine that runs. */
export function startingAgent(
  harnesses: readonly Harness[],
): HarnessRef | null {
  const known = knownOf(harnesses);
  const kind =
    (known.find(({ harness }) => runsTimeline(harness)) ?? known[0])?.kind ??
    null;
  return (
    kind ??
    harnesses.find((harness) => agentModes(harness).kind !== "neither")?.kind ??
    null
  );
}

/** The agent the new-session page shows: the person's choice while the
 * machine still lists it, else the one kept for the machine
 * (`remembered`, one of the four), else `startingAgent`. */
export function chosenAgent(
  wanted: HarnessRef | null,
  harnesses: readonly Harness[],
  remembered: HarnessKind | null,
): HarnessRef | null {
  const listed = (agent: HarnessRef | null): HarnessRef | null =>
    harnesses.find((harness) => sameAgent(harness.kind, agent))?.kind ?? null;
  return listed(wanted) ?? listed(remembered) ?? startingAgent(harnesses);
}

/** What the page says when a machine will not start `agent` now
 * (`conflict` from `harness.options`, ADR 0032): one of the four while an
 * install of it runs there, or under a profile whose model route has no
 * key there (ADR 0015); an added agent while a secret its definition
 * names is not set there. Said from what the page knows, not from the
 * machine's message, which is written for the CLI. */
export function refusedStart(
  agent: HarnessRef,
  agents: readonly AgentInfo[],
): string {
  const name = agentName(agent, agents);
  if (knownAgent(agent) !== null) {
    return `${name} cannot start on this machine now: an install of it is running there, or the profile's model route has no key there. Try again once the install ends, or choose another profile.`;
  }
  const info = addedAgents(agents).find((entry) =>
    sameAgent(entry.agent, agent),
  );
  const secrets =
    info?.detail.kind === "added"
      ? info.detail.added.secrets.map((entry) => entry.secret)
      : [];
  const named =
    secrets.length === 0
      ? "a secret"
      : secrets.length === 1
        ? `the secret ${secrets.join("")}`
        : `one of the secrets ${secrets.join(", ")}`;
  return `${name} cannot start on this machine now: it needs ${named}, which is not set there. Set it on that machine (vessel fleet secret set NAME), then try again.`;
}

/** The machine's harnesses that are one of the four, with which. */
function knownOf(
  harnesses: readonly Harness[],
): { harness: Harness; kind: HarnessKind }[] {
  return harnesses.flatMap((harness) => {
    const kind = knownAgent(harness.kind);
    return kind === null ? [] : [{ harness, kind }];
  });
}

/** One profile to choose: `""` is the agent's default. */
export interface ProfileChoice {
  value: string;
  label: string;
  detail?: string;
}

/** The profiles of `agent` on a machine, after its default. A profile on
 * a model route says which (ADR 0015): choosing it is how a session runs
 * on another provider's model. */
export function profileChoices(
  profiles: readonly ProfileInfo[],
  agent: HarnessRef,
): ProfileChoice[] {
  return [
    { value: "", label: "Default" },
    ...profiles
      .filter((profile) => sameAgent(profile.harness, agent))
      .map((profile) => {
        const route = profile.model ?? null;
        return {
          value: profile.name,
          label: profile.label ?? profile.name,
          ...(route === null ? {} : { detail: route }),
        };
      }),
  ];
}

/** One profile a session can move to, and whether it may (ADR 0015). */
export type MoveChoice = ProfileChoice & { disabled: boolean };

/** Where a session of `agent` may move: its profiles as `profileChoices`
 * lists them, each off unless it is on the route the session runs on, so
 * the default (no route) only when the session has none. The node refuses
 * the others; the list says so before it is asked. */
export function moveChoices(
  profiles: readonly ProfileInfo[],
  agent: HarnessRef,
  route: string | null,
): MoveChoice[] {
  const routeOf = (name: string): string | null =>
    name === ""
      ? null
      : (profiles.find((profile) => profile.name === name)?.model ?? null);
  return profileChoices(profiles, agent).map((choice) => ({
    ...choice,
    disabled: routeOf(choice.value) !== route,
  }));
}
