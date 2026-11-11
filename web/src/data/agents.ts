// Agents in the page's words (milestone 10, "Words on the page"): the
// protocol's harness is the agent a person chose, called by its product
// name, not the command that starts it.

import type { Harness, HarnessKind, ProfileInfo } from "../wire/proto";

/** What the page calls an agent. */
export function agentName(kind: HarnessKind): string {
  switch (kind) {
    case "claude-code":
      return "Claude Code";
    case "codex":
      return "Codex";
    case "gemini":
      return "Gemini";
    case "open-code":
      return "OpenCode";
  }
}

/** Whether `harness` can run a session with a timeline (and so has
 * options to choose); every installed one runs in a terminal. */
export function runsTimeline(harness: Harness): boolean {
  switch (harness.structured.state) {
    case "available":
      return true;
    case "unavailable":
      return false;
  }
}

/** One agent to choose, in the kit's choice shape. */
export interface AgentChoice {
  value: HarnessKind;
  label: string;
  detail?: string;
}

/** The agents a machine has, each with its version as the detail. */
export function agentChoices(harnesses: readonly Harness[]): AgentChoice[] {
  return harnesses.map((harness) => {
    const version = harness.version?.split(/\s+/)[0] ?? null;
    return {
      value: harness.kind,
      label: agentName(harness.kind),
      ...(version === null || version === "" ? {} : { detail: version }),
    };
  });
}

/** The agent a new session starts with: the first that runs with a
 * timeline, else the first installed. */
export function startingAgent(
  harnesses: readonly Harness[],
): HarnessKind | null {
  return (harnesses.find(runsTimeline) ?? harnesses[0])?.kind ?? null;
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
  agent: HarnessKind,
): ProfileChoice[] {
  return [
    { value: "", label: "Default" },
    ...profiles
      .filter((profile) => profile.harness === agent)
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
