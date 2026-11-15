// An entry of a structured session's timeline as one line of text: what
// the timeline's live region says when an entry arrives or grows
// (features/session/timeline.tsx). The words are the ones the entry's
// row shows (timeline-item.tsx), without its markup.
//
// The list itself cannot be the live region since it is windowed
// (features/session/use-window.ts): rows drawn as the reader scrolls
// would be announced as if they were new.

import { commandStatus } from "./native-command";
import { settled, settledLine } from "./approval";
import { turnTokens } from "./context-meter";
import type { Entry } from "./timeline-model";

/** `entry` as a line to announce. */
export function spoken(entry: Entry): string {
  switch (entry.kind) {
    case "command":
      return `${entry.text} · ${commandStatus(entry.item.status)}`;
    case "ready":
      return "Agent ready";
    case "prompt":
      return `Prompt: ${entry.text}`;
    case "steer":
      return `Steered: ${entry.text}`;
    case "question":
      return entry.outcome === null
        ? `The agent asks: ${entry.question.message}`
        : questionLine(entry.outcome);
    case "message":
      return entry.text;
    case "thought":
      return "Thought";
    case "tool":
      return `${entry.tool.title} · ${entry.tool.status.replace("_", " ")}`;
    case "plan": {
      const done = entry.entries.filter(
        (item) => item.status === "completed",
      ).length;
      return `Plan · ${String(done)} of ${String(entry.entries.length)} done`;
    }
    case "permission":
      return settledLine(
        entry.title,
        settled(entry.request.options, entry.outcome),
      );
    case "turn_ended":
      return entry.usage === null
        ? `Turn ended · ${entry.reason.replace("_", " ")}`
        : `Turn ended · ${entry.reason.replace("_", " ")} · ${turnTokens(entry.usage)}`;
    case "turn_failed":
      return `Turn failed · ${entry.message}`;
    case "notification":
      return `${entry.item.level}: ${entry.item.message}`;
    case "error":
      return entry.message;
    case "compaction":
      return entry.item.status === "loading"
        ? "Compacting context"
        : "Context compacted";
    case "plugin":
      return `${entry.item.plugin} · ${entry.item.kind}`;
    case "log":
      return entry.line;
    case "status":
      return `Session ${entry.status.state}`;
    case "profile":
      return `Profile changed from ${entry.from ?? "default"} to ${entry.to ?? "default"}`;
    case "model":
      return `Model call · ${entry.call.route} · ${entry.call.model}`;
  }
}

/** How a question was settled, as its line says. */
export function questionLine(
  outcome: Exclude<Extract<Entry, { kind: "question" }>["outcome"], null>,
): string {
  switch (outcome.outcome) {
    case "answered":
      return "Answered";
    case "declined":
      return "Skipped";
    case "cancelled":
      return "Question closed";
  }
}
