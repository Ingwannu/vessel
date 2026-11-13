// Agents in the page's words (milestone 10, "Words on the page"): the
// protocol's harness is the agent a person chose, called by its product
// name, not the command that starts it. The one place an agent is named
// and its kinds are listed; what the new-session page chooses from them
// is `agent-choices.ts`, what Settings says of them `agent-status.ts`.
//
// Since protocol 11 a node may also run an agent its owner added by its
// command (ADR 0032): `HarnessRef` is one of the four or `{ added: name }`.
// The page draws nothing new for those yet (the agents' page track will):
// it shows one by the name the node gives it wherever a session of one
// appears, and offers only the four where it offers a choice.

import type { Harness, HarnessKind, HarnessRef } from "../wire/proto";

/** Every agent Vessel can drive, in the order the page lists them. */
export const AGENT_KINDS: readonly HarnessKind[] = [
  "claude-code",
  "codex",
  "gemini",
  "open-code",
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

/** What the page calls an agent: the four by their names as their makers
 * write them, an added one by the name its node gives it. */
export function agentName(agent: HarnessRef): string {
  return typeof agent === "string" ? knownName(agent) : agent.added;
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
  }
}

/** The one letter an agent's mark shows (the kit's Avatar). A letter,
 * not the vendor's logo: the logos are theirs, and a board needs only to
 * tell the four apart at a glance. (Claude Code and Codex both start with
 * C.) */
export function agentMark(agent: HarnessRef): string {
  return typeof agent === "string"
    ? knownMark(agent)
    : agent.added.slice(0, 1).toUpperCase();
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
  }
}

/** `value` as one of the agents, if it names one (an address's
 * `?agent=`). */
export function agentKindOf(value: string | undefined): HarnessKind | null {
  return AGENT_KINDS.find((kind) => kind === value) ?? null;
}

/** Whether `harness` can run a session with a timeline (and so has
 * options to choose); every one of the four installed runs in a
 * terminal, an added agent does not (`runsTerminal`). */
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
