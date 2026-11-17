// The context meter beside the agent's control (milestone 11, Track
// A-page): agent-orchestrator's ring, filled by how much of the context
// the agent says is used, its colour by how close it is to full, and its
// words on hover; and a turn's tokens, as the timeline's turn line says
// them.
//
// The figures are the agent's own (`usage_update` and the prompt's
// answer): Vessel neither counts tokens nor guesses a window the agent
// did not report.

import type { ContextUsage, TurnUsage } from "../wire/proto";

/** How full the context is, by agent-orchestrator's thresholds (70% and
 * 90%). */
export type Fullness = "room" | "low" | "full";

export interface Meter {
  /** Used over size, 0 to 1. */
  fraction: number;
  /** What the ring draws: never under 8%, so a nearly empty context
   * still shows a sliver (the reference's). */
  arc: number;
  percent: number;
  fullness: Fullness;
  /** The hover's words as Paseo's context-window-details.tsx sets them:
   * the title, the share used (foreground), then muted details (the
   * tokens, the session's cost, a warning when room runs low). */
  title: string;
  used: string;
  details: readonly string[];
}

/** A token count as the meter writes it: 950, 84k, 1.2M. */
export function tokens(count: number): string {
  if (count < 1000) {
    return String(count);
  }
  if (count < 1_000_000) {
    return `${String(Math.round(count / 1000))}k`;
  }
  const millions = count / 1_000_000;
  return `${millions < 10 ? millions.toFixed(1) : String(Math.round(millions))}M`;
}

/** The meter for `usage`; `null` before the agent reported one, or when
 * it reported no window. */
export function meterOf(usage: ContextUsage | null): Meter | null {
  if (usage === null || usage.size <= 0) {
    return null;
  }
  const fraction = Math.min(1, Math.max(0, usage.used / usage.size));
  const percent = Math.round(fraction * 100);
  const fullness: Fullness =
    fraction >= 0.9 ? "full" : fraction >= 0.7 ? "low" : "room";
  const details = [`${tokens(usage.used)} / ${tokens(usage.size)} tokens`];
  const cost = usage.cost;
  if (cost !== undefined && cost !== null) {
    details.push(`Session cost ${costLabel(cost.amount, cost.unit)}`);
  }
  switch (fullness) {
    case "room":
      break;
    case "low":
      details.push("Room is running low. A long task may not fit.");
      break;
    case "full":
      details.push(
        "The next turn may not fit. Compacting or starting a new conversation will reclaim room.",
      );
      break;
  }
  return {
    fraction,
    arc: Math.max(0.08, fraction),
    percent,
    fullness,
    title: "Context window",
    used: `${String(percent)}% used`,
    details,
  };
}

/** An amount in its unit: "$0.50" for dollars, "12.5 credits" else. */
export function costLabel(amount: number, unit: string): string {
  return unit.toUpperCase() === "USD"
    ? `$${amount.toFixed(2)}`
    : `${String(Number(amount.toFixed(4)))} ${unit}`;
}

/** A turn's tokens, for the turn line: "30 tokens · 20 in · 10 out". */
export function turnTokens(usage: TurnUsage): string {
  const count = (value: number) => value.toLocaleString("en-US");
  const parts = [
    `${count(usage.total)} tokens`,
    `${count(usage.input)} in`,
    `${count(usage.output)} out`,
  ];
  const cached = usage.cached_read;
  if (cached !== undefined && cached !== null && cached > 0) {
    parts.push(`${count(cached)} cached`);
  }
  return parts.join(" · ");
}
