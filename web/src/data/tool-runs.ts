// "Collapse tool calls" (Settings › Appearance; Paseo's "Chat display"):
// a run of consecutive tool calls and thoughts is one summary row, "Edited
// 2 files and ran 3 commands", opened on a click (DESIGN.md §6.5).
//
// A pure function of the entries, so the timeline draws what it says and
// a windowed timeline can ask the same of any slice.

import type { ToolKind } from "../wire/proto";
import type { Entry } from "./timeline-model";

/** One run of consecutive tool calls and thoughts. */
export interface ToolRun {
  /** The first entry's key: where the summary row stands. */
  lead: string;
  /** Every entry of the run, the lead first. */
  members: readonly string[];
  /** Paseo's sentence for it. */
  summary: string;
}

/** What the timeline needs to fold: the run each folded entry is in. */
export type Folds = ReadonlyMap<string, ToolRun>;

type Tally = "edited" | "ran" | "read" | "searched" | "other";

function tallyOf(kind: ToolKind): Tally {
  switch (kind) {
    case "edit":
    case "delete":
    case "move":
      return "edited";
    case "execute":
      return "ran";
    case "read":
      return "read";
    case "search":
      return "searched";
    case "think":
    case "fetch":
    case "switch_mode":
    case "other":
      return "other";
  }
}

function plural(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

function phrase(tally: Tally, count: number): string {
  switch (tally) {
    case "edited":
      return `edited ${plural(count, "file", "files")}`;
    case "ran":
      return `ran ${plural(count, "command", "commands")}`;
    case "read":
      return `read ${plural(count, "file", "files")}`;
    case "searched":
      return `searched ${plural(count, "time", "times")}`;
    case "other":
      return `used ${plural(count, "other tool", "other tools")}`;
  }
}

/** The sentence for a run's tool kinds (`thoughts`: how many thoughts it
 * holds). */
export function runSummary(
  kinds: readonly ToolKind[],
  thoughts: number,
): string {
  if (kinds.length === 0) {
    return thoughts > 0 ? "Thought" : "";
  }
  const counts = new Map<Tally, number>();
  for (const kind of kinds) {
    const tally = tallyOf(kind);
    counts.set(tally, (counts.get(tally) ?? 0) + 1);
  }
  const order: Tally[] = ["edited", "ran", "read", "searched", "other"];
  const parts = order
    .filter((tally) => counts.has(tally))
    .map((tally) => phrase(tally, counts.get(tally) ?? 0));
  const last = parts.pop() ?? "";
  const text = parts.length === 0 ? last : `${parts.join(", ")} and ${last}`;
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}`;
}

/** The runs of `entries` worth folding (two entries or more), by every
 * member's key. */
export function toolRuns(entries: readonly Entry[]): Folds {
  const folds = new Map<string, ToolRun>();
  let run: Entry[] = [];
  const close = () => {
    if (run.length >= 2) {
      const kinds = run.flatMap((entry) =>
        entry.kind === "tool" ? [entry.tool.kind] : [],
      );
      const thoughts = run.filter((entry) => entry.kind === "thought").length;
      const folded: ToolRun = {
        lead: run[0]?.key ?? "",
        members: run.map((entry) => entry.key),
        summary: runSummary(kinds, thoughts),
      };
      for (const entry of run) {
        folds.set(entry.key, folded);
      }
    }
    run = [];
  };
  for (const entry of entries) {
    if (entry.kind === "tool" || entry.kind === "thought") {
      run.push(entry);
    } else {
      close();
    }
  }
  close();
  return folds;
}
