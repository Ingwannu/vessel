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

import type { Harness, HarnessKind } from "../wire/proto";
import { AGENT_KINDS } from "./agents";
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
