// StatusDot (DESIGN.md §6.13): an 8px dot in a state's colour, named for
// a screen reader by its word (state is never colour alone, §4.3).

import "./status-dot.css";

import type { Tone } from "./badge";

function dotClass(tone: Tone): string {
  switch (tone) {
    case "ok":
      return "status-dot";
    case "warn":
      return "status-dot warn";
    case "danger":
      return "status-dot danger";
    case "interrupt":
      return "status-dot interrupt";
    case "neutral":
      return "status-dot offline";
  }
}

export function StatusDot({ tone, label }: { tone: Tone; label: string }) {
  return <span className={dotClass(tone)} aria-label={label} role="img" />;
}
