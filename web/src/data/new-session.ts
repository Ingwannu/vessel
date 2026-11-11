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

import type { BookNode, NodeKey, NodeStatus } from "../wire/bridge";
import type {
  Harness,
  HarnessKind,
  SessionOption,
  SessionSpec,
} from "../wire/proto";
import { runsTimeline } from "./agents";
import { choicesFor, type Picks } from "./options";
import { titleFromPrompt } from "./titles";

/** What the person has chosen so far. */
export interface Draft {
  machine: NodeKey | null;
  agent: HarnessKind | null;
  /** `""`: the agent's default profile. */
  profile: string;
  terminal: boolean;
  /** A terminal's arguments, as typed. */
  args: string;
  /** `""`: the machine's home folder. */
  folder: string;
  prompt: string;
  picks: Picks;
}

/** What the page knows of the agent's options. */
export type OptionsState =
  | { kind: "loading" }
  | { kind: "ready"; options: readonly SessionOption[] }
  | { kind: "failed"; message: string };

/** A terminal's size until its view measures itself. */
const FIRST_SIZE = { cols: 120, rows: 36 };

/** The machine a new session starts on: the one asked for, else the one
 * last chosen, else the book's default, else the first; only a connected
 * one can take a session. */
export function startingMachine(
  book: readonly BookNode[],
  statuses: Readonly<Record<NodeKey, NodeStatus>>,
  wanted: NodeKey | null,
  selected: NodeKey | null,
): NodeKey | null {
  const connected = (key: NodeKey | null): key is NodeKey =>
    key !== null && statuses[key]?.state === "connected";
  const candidates = [
    wanted,
    selected,
    book.find((node) => node.default)?.key ?? null,
    ...book.map((node) => node.key),
  ];
  return candidates.find(connected) ?? null;
}

/** Whether the draft starts a terminal: asked for, or the only way the
 * agent runs on that machine. */
export function startsTerminal(draft: Draft, harness: Harness): boolean {
  return draft.terminal || !runsTimeline(harness);
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
  return {
    harness: harness.kind,
    mode: "structured",
    model: null,
    ...(choices.length === 0 ? {} : { options: choices }),
    ...where,
  };
}

/** Whether Send can go, and if not, why (`null`: nothing to say, as for
 * an empty box). */
export function sendable(
  draft: Draft,
  harness: Harness | null,
): { ok: true } | { ok: false; reason: string | null } {
  if (draft.machine === null) {
    return { ok: false, reason: "No machine is connected." };
  }
  if (harness === null) {
    return { ok: false, reason: "This machine has no agent installed." };
  }
  if (!startsTerminal(draft, harness) && draft.prompt.trim() === "") {
    return { ok: false, reason: null };
  }
  return { ok: true };
}
