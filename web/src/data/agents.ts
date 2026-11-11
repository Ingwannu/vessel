// Agents as Settings shows them (track W2): each agent a machine could
// run, its status as GOAT's provider card says it, and what it offers
// once asked (`harness.options`, ADR 0022), grouped in the page's words.
//
// [Decision Log]
// - Purpose: the owner found nothing on the page that said which agents a
//   machine has, nor what they offer.
// - Constraints: `harness.list` names only what the machine found
//   installed; `harness.options` starts the agent on that machine to ask
//   (no prompt is sent, but it is a process and a login check), so it is
//   asked for one agent when its card is opened, never for all at once.
//   Nothing is invented: what an agent does not offer is not shown.
// - Choice: every agent Vessel can drive is listed, so "Not found" is
//   said rather than an agent silently missing; an installed one that
//   cannot run a conversation (no ACP adapter) "Needs attention" with the
//   machine's reason; the rest are "Available". What it offers is the
//   agent's own options, grouped by ACP's category under the plan's words:
//   models, effort, permissions, then the rest.

import type { NodeKey } from "../wire/bridge";
import type {
  Harness,
  HarnessKind,
  HarnessOptions,
  OptionCategory,
  SessionOption,
} from "../wire/proto";
import type { StateTone } from "./format";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** Every agent Vessel can drive, in the order the page lists them. */
export const AGENT_KINDS: readonly HarnessKind[] = [
  "claude-code",
  "codex",
  "gemini",
  "open-code",
];

/** An agent's name, as its makers write it. */
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

/** An agent's status on one machine (GOAT's provider headlines). */
export type AgentStatus =
  | { kind: "available"; harness: Harness }
  | { kind: "terminal-only"; harness: Harness; reason: string }
  | { kind: "not-found" };

/** What `kind` is on a machine that found `harnesses`. */
export function agentStatus(
  kind: HarnessKind,
  harnesses: readonly Harness[],
): AgentStatus {
  const harness = harnesses.find((found) => found.kind === kind);
  if (harness === undefined) {
    return { kind: "not-found" };
  }
  return harness.structured.state === "available"
    ? { kind: "available", harness }
    : {
        kind: "terminal-only",
        harness,
        reason: harness.structured.reason,
      };
}

/** The headline, its tone, and the line under it. */
export function agentHeadline(status: AgentStatus): {
  headline: string;
  tone: StateTone;
  detail: string | null;
} {
  switch (status.kind) {
    case "available":
      return { headline: "Available", tone: "ok", detail: null };
    case "terminal-only":
      return {
        headline: "Needs attention",
        tone: "warn",
        detail: `Runs in a terminal only: ${status.reason}`,
      };
    case "not-found":
      return {
        headline: "Not found",
        tone: "neutral",
        detail: "Not installed on this machine",
      };
  }
}

/** The agents on a machine, installed first, each in `AGENT_KINDS`'
 * order. */
export function agentsOn(
  harnesses: readonly Harness[],
): { kind: HarnessKind; status: AgentStatus }[] {
  const all = AGENT_KINDS.map((kind) => ({
    kind,
    status: agentStatus(kind, harnesses),
  }));
  return [
    ...all.filter((agent) => agent.status.kind !== "not-found"),
    ...all.filter((agent) => agent.status.kind === "not-found"),
  ];
}

/** `value` as one of the agents, if it names one. */
export function agentKindOf(value: string | undefined): HarnessKind | null {
  return AGENT_KINDS.find((kind) => kind === value) ?? null;
}

/** The groups of what an agent offers, in the page's words. */
export type OfferGroup = "models" | "effort" | "permissions" | "other";

/** Which group an option of `category` is in. */
export function offerGroupOf(category: OptionCategory): OfferGroup {
  switch (category) {
    case "model":
      return "models";
    case "thought_level":
      return "effort";
    case "mode":
      return "permissions";
    case "model_config":
    case "other":
      return "other";
  }
}

/** What a group is called on the page. */
export function offerGroupLabel(group: OfferGroup): string {
  switch (group) {
    case "models":
      return "Models";
    case "effort":
      return "Effort";
    case "permissions":
      return "Permissions";
    case "other":
      return "Other options";
  }
}

/** One option as a row: its name, its values (or on/off), and the one in
 * use now. An option of a kind this page does not know is left out, as
 * ADR 0022 asks of a client. */
export interface OfferRow {
  id: string;
  name: string;
  values: string[];
  current: string;
}

function offerRow(option: SessionOption): OfferRow | null {
  switch (option.kind) {
    case "select":
      return {
        id: option.id,
        name: option.name,
        values: option.values.map((value) => value.name),
        current:
          option.values.find((value) => value.id === option.current)?.name ??
          option.current,
      };
    case "toggle":
      return {
        id: option.id,
        name: option.name,
        values: ["On", "Off"],
        current: option.on ? "On" : "Off",
      };
    case "unknown":
      return null;
  }
}

/** What an agent offers, by group in the page's order; groups it offers
 * nothing in are left out. */
export function offerGroups(
  options: readonly SessionOption[],
): { group: OfferGroup; rows: OfferRow[] }[] {
  const order: readonly OfferGroup[] = [
    "models",
    "effort",
    "permissions",
    "other",
  ];
  return order
    .map((group) => ({
      group,
      rows: options
        .filter((option) => offerGroupOf(option.category) === group)
        .map(offerRow)
        .filter((row) => row !== null),
    }))
    .filter((entry) => entry.rows.length > 0);
}

/** Ask `node` what `kind` offers. The machine starts the agent to ask
 * (no prompt is sent), or answers from an answer under ten minutes old. */
export function loadOffer(
  fleet: Fleet,
  node: NodeKey,
  kind: HarnessKind,
): Promise<Outcome<HarnessOptions>> {
  return attempt(() =>
    fleet.bridge.call(node, "harness.options", { harness: kind }),
  );
}
