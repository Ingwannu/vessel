// A window's short label in the sidebar summary (Paseo `shortLabel`): the
// source's own, hidden when it says so, else "5h" and "wk" for the node's
// standard windows (as on the native app), else the whole label.
import type { UsageWindow } from "../../wire/proto";

export function shortLabel(window: UsageWindow): string {
  const own = window.short_label;
  if (own !== null) return own.kind === "hidden" ? "" : own.text;
  const period = window.id.startsWith("general:")
    ? window.id.slice("general:".length)
    : window.id;
  if (period === "five_hour" && window.label === "5-hour window") return "5h";
  if (period === "weekly" && window.label === "Weekly window") return "wk";
  return window.label;
}
