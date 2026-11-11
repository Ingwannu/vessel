// Agents in the page's words (milestone 10, "Words on the page"): the
// protocol's harness is the agent a person chose, called by its product
// name, not the command that starts it. The one place an agent is named
// and its kinds are listed; what the new-session page chooses from them
// is `agent-choices.ts`, what Settings says of them `agent-status.ts`.

import type { Harness, HarnessKind } from "../wire/proto";

/** Every agent Vessel can drive, in the order the page lists them. */
export const AGENT_KINDS: readonly HarnessKind[] = [
  "claude-code",
  "codex",
  "gemini",
  "open-code",
];

/** What the page calls an agent: its name as its makers write it. */
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

/** The one letter an agent's mark shows (the kit's Avatar). A letter,
 * not the vendor's logo: the logos are theirs, and a board needs only to
 * tell the four apart at a glance. (Claude Code and Codex both start with
 * C.) */
export function agentMark(kind: HarnessKind): string {
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
 * options to choose); every installed one runs in a terminal. */
export function runsTimeline(harness: Harness): boolean {
  switch (harness.structured.state) {
    case "available":
      return true;
    case "unavailable":
      return false;
  }
}
