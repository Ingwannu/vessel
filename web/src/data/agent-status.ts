// What Settings says of a machine's agents (track W2): each agent's
// status as GOAT's provider card says it, and what it offers once asked
// (`harness.options`, ADR 0022), as one list.
//
// [Decision Log]
// - Purpose: the owner found nothing on the page that said which agents a
//   machine has, nor what they offer.
// - Constraints: `harness.list` names only what the machine found
//   installed; `harness.options` starts the agent on that machine to ask
//   (no prompt is sent, but it is a process and a login check), so it is
//   asked for one agent when its card is opened, never for all at once
//   (`calls.ts`' `probeOptions`, the same call the new-session page
//   makes). Nothing is invented: what an agent does not offer is not
//   shown.
// - Choice: every agent Vessel can drive is listed, so "Not found" is
//   said rather than an agent silently missing; an installed one that
//   cannot run a conversation (no ACP adapter) "Needs attention" with the
//   machine's reason; the rest are "Available". What it offers is laid
//   out by `options.ts`' `controlsOf`, the composer's own reading, so the
//   card and the composer place and name every option alike.
// - Protocol 11 (ADR 0032): the agents added on a machine are listed
//   after the four it has, by their label: one whose program the machine
//   found (it is in `harness.list`) is "Available" as a conversation
//   without a terminal; one it did not (only in `agent.list`) is "Not
//   found", naming its program. Adding, removing and installing them are
//   a later track's screens.

import type { AgentInfo, Harness, HarnessRef } from "../wire/proto";
import {
  AGENT_KINDS,
  addedAgents,
  agentKey,
  agentModes,
  knownAgent,
  sameAgent,
} from "./agents";
import { ago, type StateTone } from "./format";
import {
  type Controls,
  currentName,
  type SelectControl,
  slotLabel,
} from "./options";

/** An agent's status on one machine (GOAT's provider headlines). */
export type AgentStatus =
  | { kind: "available"; harness: Harness }
  | { kind: "terminal-only"; harness: Harness; reason: string }
  /** Not found; for an added agent, the program its machine looked for. */
  | { kind: "not-found"; program: string | null };

/** What `agent` is on a machine that found `harnesses`; `program` is an
 * added agent's, from `agent.list`. */
export function agentStatus(
  agent: HarnessRef,
  harnesses: readonly Harness[],
  program: string | null = null,
): AgentStatus {
  const harness = harnesses.find((found) => sameAgent(found.kind, agent));
  if (harness === undefined) {
    return { kind: "not-found", program };
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
      return {
        headline: "Available",
        tone: "ok",
        detail:
          agentModes(status.harness).kind === "timeline"
            ? "Runs as a conversation; it has no terminal"
            : null,
      };
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
        detail:
          status.program === null
            ? "Not installed on this machine"
            : `Its program, ${status.program}, is not found on this machine`,
      };
  }
}

/** The agents on a machine, found before not found: the four in
 * `AGENT_KINDS`' order, the agents added on it after them (from
 * `agent.list` when read, and those `harness.list` found). */
export function agentsOn(
  harnesses: readonly Harness[],
  agents: readonly AgentInfo[] = [],
): { agent: HarnessRef; status: AgentStatus }[] {
  const four = AGENT_KINDS.map((kind) => ({
    agent: kind,
    status: agentStatus(kind, harnesses),
  }));
  const listed = addedAgents(agents).map((info) => ({
    agent: info.agent,
    status: agentStatus(
      info.agent,
      harnesses,
      info.detail.kind === "added" ? info.detail.added.program : null,
    ),
  }));
  const keys = new Set(listed.map((entry) => agentKey(entry.agent)));
  const found = harnesses
    .filter(
      (harness) =>
        knownAgent(harness.kind) === null && !keys.has(agentKey(harness.kind)),
    )
    .map((harness) => ({
      agent: harness.kind,
      status: agentStatus(harness.kind, harnesses),
    }));
  const added = [...listed, ...found];
  const isFound = (entry: { status: AgentStatus }): boolean =>
    entry.status.kind !== "not-found";
  return [
    ...four.filter(isFound),
    ...added.filter(isFound),
    ...added.filter((entry) => !isFound(entry)),
    ...four.filter((entry) => !isFound(entry)),
  ];
}

/** One option as a row of the card: what the page calls it, its values
 * (or on/off), and the one in use now. */
export interface OfferRow {
  id: string;
  label: string;
  values: string[];
  current: string;
}

function selectRow(control: SelectControl, label: string): OfferRow {
  return {
    id: control.id,
    label,
    values: control.values.map((value) => value.name),
    current: currentName(control),
  };
}

/** What an agent offers, as the card lists it: the model, the effort and
 * the permissions in the page's words, then the switches and any other
 * choice in the agent's own. */
export function offerRows(controls: Controls): OfferRow[] {
  return [
    ...(controls.model === null
      ? []
      : [selectRow(controls.model, slotLabel("model"))]),
    ...(controls.effort === null
      ? []
      : [selectRow(controls.effort, slotLabel("effort"))]),
    ...(controls.permissions === null
      ? []
      : [selectRow(controls.permissions, slotLabel("permissions"))]),
    ...controls.toggles.map((toggle) => ({
      id: toggle.id,
      label: toggle.name,
      values: ["On", "Off"],
      current: toggle.on ? "On" : "Off",
    })),
    ...controls.selects.map((control) => selectRow(control, control.name)),
  ];
}

/** When the machine asked the agent, as the card's note says it. */
export function askedWhen(probedAt: string, now: number = Date.now()): string {
  const seconds = (now - Date.parse(probedAt)) / 1000;
  return Number.isNaN(seconds) || seconds < 60
    ? "Asked just now"
    : `Asked ${ago(probedAt, now)} ago`;
}
