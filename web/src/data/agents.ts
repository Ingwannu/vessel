// Agents in the page's words (milestone 10, "Words on the page"): the
// protocol's harness is the agent a person chose, called by its product
// name, not the command that starts it. The one place an agent is named
// and its kinds are listed; what the new-session page chooses from them
// is `agent-choices.ts`, what Settings says of them `agent-status.ts`.
//
// [Decision Log]
// - Purpose: since protocol 11 a node may run an agent its owner added by
//   its command (ADR 0032): `HarnessRef` is one of the four or `{ added:
//   name }`. The page must be honest with a machine that has some: show
//   them wherever agents are chosen or shown, by their label.
// - Constraint: `harness.list` carries an added agent only when its
//   program is found, with no label; `agent.list` carries every added
//   agent with its label (protocol 11 only). Two `HarnessRef`s are objects
//   for added agents, so `===` tells nothing.
// - Choice: compare by `agentKey` only; name an added agent by its label
//   from `agent.list` when the page has read it, else by the name the
//   machine gives it; its mark is the shown name's first letter. What a
//   machine lets an agent run as (timeline, terminal, both, neither) is
//   `agentModes`, read from `harness.list`'s own facts.
// - Cost: a label shows once `agent.list` is read (the fleet reads it only
//   for a machine that shows an added agent, `fleet.ts`); until then the
//   name.

import type {
  AgentInfo,
  Harness,
  HarnessKind,
  HarnessRef,
} from "../wire/proto";

/** Every agent Vessel can drive, in the order the page lists them: the
 * four, then the seven of ADR 0058 in the order the owner
 * named them. */
export const AGENT_KINDS: readonly HarnessKind[] = [
  "claude-code",
  "codex",
  "gemini",
  "open-code",
  "grok",
  "pi",
  "omp",
  "antigravity",
  "muse",
  "amp",
  "cursor",
];

/** One of the four, or `null` for an agent added on a node. */
export function knownAgent(agent: HarnessRef): HarnessKind | null {
  return typeof agent === "string" ? agent : null;
}

/** A string that tells agents apart: the four's wire names, and
 * `added:<name>` (never a wire name). Two `HarnessRef`s are compared by
 * it, never as objects. */
export function agentKey(agent: HarnessRef): string {
  return typeof agent === "string" ? agent : `added:${agent.added}`;
}

/** Whether `a` and `b` are one agent. */
export function sameAgent(a: HarnessRef | null, b: HarnessRef | null): boolean {
  return a === null || b === null ? a === b : agentKey(a) === agentKey(b);
}

/** What the page calls an agent: the four by their names as their makers
 * write them, an added one by its label in `agents` (the machine's
 * `agent.list`), else by the name its machine gives it. */
export function agentName(
  agent: HarnessRef,
  agents: readonly AgentInfo[] = [],
): string {
  if (typeof agent === "string") {
    return knownName(agent);
  }
  const info = agents.find((entry) => sameAgent(entry.agent, agent));
  const label =
    info?.detail.kind === "added" ? (info.detail.added.label ?? null) : null;
  return label ?? agent.added;
}

function knownName(kind: HarnessKind): string {
  switch (kind) {
    case "claude-code":
      return "Claude Code";
    case "codex":
      return "Codex";
    case "gemini":
      return "Gemini";
    case "open-code":
      return "OpenCode";
    case "grok":
      return "Grok";
    case "pi":
      return "Pi";
    case "omp":
      return "Oh My Pi";
    case "antigravity":
      return "Antigravity";
    case "muse":
      return "Muse Code";
    case "amp":
      return "Amp";
    case "cursor":
      return "Cursor";
  }
}

/** The one letter an agent's mark shows (the kit's Avatar). A letter,
 * not the vendor's logo: the logos are theirs, and a board needs only to
 * tell the four apart at a glance. (Claude Code and Codex both start with
 * C.) The seven of ADR 0058 show the generic mark, their name's first
 * letter, as an added agent does, until their vendors' marks are cleared. */
export function agentMark(
  agent: HarnessRef,
  agents: readonly AgentInfo[] = [],
): string {
  return typeof agent === "string"
    ? knownMark(agent)
    : agentName(agent, agents).slice(0, 1).toUpperCase();
}

function knownMark(kind: HarnessKind): string {
  switch (kind) {
    case "claude-code":
      return "C";
    case "codex":
      return "X";
    case "gemini":
      return "G";
    case "open-code":
      return "O";
    case "grok":
    case "pi":
    case "omp":
    case "antigravity":
    case "muse":
    case "amp":
    case "cursor":
      return knownName(kind).slice(0, 1).toUpperCase();
  }
}

/** `value` as one of the agents, if it names one (an address's
 * `?agent=`). */
export function agentKindOf(value: string | undefined): HarnessKind | null {
  return AGENT_KINDS.find((kind) => kind === value) ?? null;
}

/** Whether `harness` can run a session with a timeline (and so has
 * options to choose). */
export function runsTimeline(harness: Harness): boolean {
  switch (harness.structured.state) {
    case "available":
      return true;
    case "unavailable":
      return false;
  }
}

/** Whether `harness` can run in a terminal: absent before protocol 11,
 * when every harness listed could. */
export function runsTerminal(harness: Harness): boolean {
  const terminal = harness.terminal ?? { state: "available" };
  switch (terminal.state) {
    case "available":
      return true;
    case "unavailable":
      return false;
  }
}

/** How a machine lets an agent run: with a timeline, in a terminal, both
 * (the four with their adapter), or neither (`harness.list` said why). */
export type AgentModes =
  | { kind: "both" }
  | { kind: "timeline"; noTerminal: string }
  | { kind: "terminal"; noTimeline: string }
  | { kind: "neither"; reason: string };

/** `harness`'s modes on its machine. */
export function agentModes(harness: Harness): AgentModes {
  const terminal = harness.terminal ?? { state: "available" };
  const { structured } = harness;
  if (structured.state === "available") {
    return terminal.state === "available"
      ? { kind: "both" }
      : { kind: "timeline", noTerminal: terminal.reason };
  }
  return terminal.state === "available"
    ? { kind: "terminal", noTimeline: structured.reason }
    : { kind: "neither", reason: structured.reason };
}

/** Whether the machine offers a terminal for the agent (`null`, not known
 * yet: as for the four). */
export function offersTerminal(modes: AgentModes | null): boolean {
  switch (modes?.kind) {
    case undefined:
    case "both":
    case "terminal":
      return true;
    case "timeline":
    case "neither":
      return false;
  }
}

/** The agents a machine has added (protocol 11's `agent.list`), out of
 * every agent it lists. */
export function addedAgents(agents: readonly AgentInfo[]): AgentInfo[] {
  return agents.filter((entry) => entry.detail.kind === "added");
}
