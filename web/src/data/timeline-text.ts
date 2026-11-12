// An entry of a structured session's timeline as one line of text: what
// the timeline's live region says when an entry arrives or grows
// (features/session/timeline.tsx). The words are the ones the entry's
// row shows (timeline-item.tsx), without its markup.
//
// The list itself cannot be the live region since it is windowed
// (features/session/use-window.ts): rows drawn as the reader scrolls
// would be announced as if they were new.

import { settled, settledLine } from "./approval";
import type { Entry } from "./timeline-model";

/** `entry` as a line to announce. */
export function spoken(entry: Entry): string {
  switch (entry.kind) {
    case "ready":
      return "Agent ready";
    case "prompt":
      return `Prompt: ${entry.text}`;
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
      return `Turn ended · ${entry.reason.replace("_", " ")}`;
    case "turn_failed":
      return `Turn failed · ${entry.message}`;
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
