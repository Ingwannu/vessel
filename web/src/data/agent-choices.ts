// What the new-session page chooses among a machine's agents (track W1):
// the agents to pick from, the one it starts with, and an agent's
// profiles, and where a running session may move among them. The agent's
// name and kinds are `agents.ts`'s.

import type {
  Harness,
  HarnessKind,
  HarnessRef,
  ProfileInfo,
} from "../wire/proto";
import { agentKey, agentName, knownAgent, runsTimeline } from "./agents";

/** One agent to choose, in the kit's choice shape. */
export interface AgentChoice {
  value: HarnessKind;
  label: string;
  detail?: string;
}

/** The four of a machine's agents, each with its version as the detail.
 * An agent added on the machine (protocol 11) is not offered here until
 * the page draws what it needs (its terminal, its defaults). */
export function agentChoices(harnesses: readonly Harness[]): AgentChoice[] {
  return knownOf(harnesses).map(({ harness, kind }) => {
    const version = harness.version?.split(/\s+/)[0] ?? null;
    return {
      value: kind,
      label: agentName(kind),
      ...(version === null || version === "" ? {} : { detail: version }),
    };
  });
}

/** The agent a new session starts with: the first of the four that runs
 * with a timeline, else the first of them installed. */
export function startingAgent(
  harnesses: readonly Harness[],
): HarnessKind | null {
  const known = knownOf(harnesses);
  return (
    (known.find(({ harness }) => runsTimeline(harness)) ?? known[0])?.kind ??
    null
  );
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
      .filter((profile) => agentKey(profile.harness) === agentKey(agent))
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
