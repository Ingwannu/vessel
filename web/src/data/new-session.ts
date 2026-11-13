// The new-session page's decisions (agent-orchestrator's `/new`): which
// machine and agent it starts on, what Send would create, and whether it
// can yet.
//
// [Decision Log]
// - Purpose: one box to describe the work, with where it runs and what
//   it runs with around it; Send creates the session, sends the prompt
//   and opens it. Milestone 9's dialog asked six questions in the
//   protocol's words first.
// - Constraint: the agent's options take seconds to read (the node
//   starts the agent to ask it, ADR 0022), and may fail (an agent not
//   logged in). The page must not wait on them: a session can always
//   start on the agent's own defaults.
// - Choice: the draft holds what the person chose; the spec is derived
//   from it here. Choices are sent only once the options are known (they
//   are made from them), and only those that change something
//   (data/options.ts). "Open as terminal" starts the agent's own TUI: no
//   options, the arguments split as a shell would, and the box's text, if
//   any, as the session's title (a terminal takes no prompt, and the node
//   names a terminal from nothing else).
// - Cost: a terminal's arguments are split here with a shell's quoting
//   rules (quotes and backslashes), not by a shell: no variables, no
//   globs. They are passed to the agent verbatim, as the CLI's are.

import type { NodeKey } from "../wire/bridge";
import type {
  Harness,
  HarnessRef,
  SessionOption,
  SessionSpec,
} from "../wire/proto";
import { agentModes, agentName } from "./agents";
import { type ControlsState, controlsOf } from "./options";
import { choicesFor, type Picks, withPicks } from "./picks";
import { titleFromPrompt } from "./titles";

/** What the person has chosen so far. */
export interface Draft {
  machine: NodeKey | null;
  /** One of the four, or an agent added on the machine (protocol 11). */
  agent: HarnessRef | null;
  /** `""`: the agent's default profile. */
  profile: string;
  terminal: boolean;
  /** A terminal's arguments, as typed. */
  args: string;
  /** `""`: the machine's home folder. */
  folder: string;
  prompt: string;
  picks: Picks;
  /** Whether the person chose a profile or an option for this agent:
   * until they do, the machine's last start's are used (defaults.ts's
   * `startingPicks`), and choosing the agent's default profile is a
   * choice too. */
  chosen: boolean;
}

/** What the page knows of the agent's options. */
export type OptionsState =
  | { kind: "loading" }
  | { kind: "ready"; options: readonly SessionOption[] }
  | { kind: "failed"; message: string }
  /** The machine will not start the agent now (`conflict`), in the
   * page's words (`agent-choices.ts`' `refusedStart`): Send would be
   * refused the same way. */
  | { kind: "refused"; message: string };

/** A terminal's size until its view measures itself. */
const FIRST_SIZE = { cols: 120, rows: 36 };

/** Whether the draft starts a terminal: asked for where the machine
 * offers one for the agent, or the only way it runs there. An agent
 * added on the machine has no terminal (ADR 0032), whatever was asked
 * for another agent before. */
export function startsTerminal(draft: Draft, harness: Harness): boolean {
  const modes = agentModes(harness);
  switch (modes.kind) {
    case "both":
      return draft.terminal;
    case "terminal":
      return true;
    case "timeline":
    case "neither":
      return false;
  }
}

/** `text` split into arguments as a shell would: on spaces, with single
 * quotes kept literal, double quotes grouping, and a backslash escaping
 * the next character. */
export function splitArgs(text: string): string[] {
  const args: string[] = [];
  let current = "";
  let started = false;
  let quote: "'" | '"' | null = null;
  let escaped = false;
  for (const char of text) {
    if (escaped) {
      current += char;
      escaped = false;
      continue;
    }
    if (char === "\\" && quote !== "'") {
      escaped = true;
      started = true;
      continue;
    }
    if (quote !== null) {
      if (char === quote) {
        quote = null;
      } else {
        current += char;
      }
      continue;
    }
    if (char === "'" || char === '"') {
      quote = char;
      started = true;
    } else if (/\s/u.test(char)) {
      if (started) {
        args.push(current);
        current = "";
        started = false;
      }
    } else {
      current += char;
      started = true;
    }
  }
  if (started) {
    args.push(current);
  }
  return args;
}

/** The spec Send creates. `options` is what the agent offers, when known;
 * until then the session starts on the agent's defaults. */
export function specOf(
  draft: Draft,
  harness: Harness,
  options: OptionsState,
): SessionSpec {
  const where = {
    ...(draft.folder.trim() === "" ? {} : { cwd: draft.folder.trim() }),
    ...(draft.profile === "" ? {} : { profile: draft.profile }),
  };
  if (startsTerminal(draft, harness)) {
    const title = titleFromPrompt(draft.prompt);
    return {
      harness: harness.kind,
      mode: "terminal",
      args: splitArgs(draft.args),
      size: FIRST_SIZE,
      ...where,
      ...(title === null ? {} : { title }),
    };
  }
  const choices =
    options.kind === "ready" ? choicesFor(options.options, draft.picks) : [];
  // Named at once by its first prompt, as GOAT and agent-orchestrator name
  // a thread at send: the node names it so only when the agent's turn
  // starts, after its start (the same rule, `titleFromPrompt`'s).
  const title = titleFromPrompt(draft.prompt);
  return {
    harness: harness.kind,
    mode: "structured",
    model: null,
    ...(choices.length === 0 ? {} : { options: choices }),
    ...where,
    ...(title === null ? {} : { title }),
  };
}

/** Whether Send can go, and if not, why (`null`: nothing to say, as for
 * an empty box). */
export function sendable(
  draft: Draft,
  harness: Harness | null,
  options: OptionsState,
  /** What the page calls the agent (an added one's label). */
  name: string | null = null,
  /** Whether the machine has listed its agents yet: before, an agent not
   * found is one not yet known, not one missing. */
  listed = true,
): { ok: true } | { ok: false; reason: string | null } {
  if (draft.machine === null) {
    return { ok: false, reason: "No machine is connected." };
  }
  if (harness === null) {
    return {
      ok: false,
      reason: listed
        ? "This machine has no agent installed."
        : "Loading the machine's agents…",
    };
  }
  const modes = agentModes(harness);
  if (modes.kind === "neither") {
    return {
      ok: false,
      reason: `${name ?? agentName(harness.kind)} cannot run on this machine: ${modes.reason}`,
    };
  }
  if (!startsTerminal(draft, harness) && options.kind === "refused") {
    return { ok: false, reason: options.message };
  }
  if (!startsTerminal(draft, harness) && draft.prompt.trim() === "") {
    return { ok: false, reason: null };
  }
  return { ok: true };
}

/** The controls the new-session page shows: the agent's options with the
 * person's picks applied, or why there are none. */
export function draftControls(
  draft: Draft,
  harness: Harness,
  options: OptionsState,
  /** What the page calls the agent (an added one's label). */
  name: string | null = null,
): ControlsState {
  if (startsTerminal(draft, harness)) {
    return { kind: "terminal" };
  }
  switch (options.kind) {
    case "loading":
      return { kind: "loading" };
    case "failed":
      return { kind: "failed", message: options.message };
    case "refused":
      return { kind: "refused", message: options.message };
    case "ready":
      return {
        kind: "ready",
        controls: controlsOf(
          withPicks(options.options, draft.picks),
          name ?? agentName(harness.kind),
        ),
      };
  }
}
