// A node's log lines as the page shows them (ADR 0111, TRACKER 569).
//
// [Decision Log]
// - Purpose: "Node logs" shows text that a person reads and pastes elsewhere.
// - Constraint: a node at the diagnostics wire's protocol already sends its
//   lines without a terminal's escapes. But a log file that was written with
//   colours (by an older build, a wrapper or a fixture) would otherwise show
//   the escape bytes, so the page drops them too (the coordinator's verdict
//   on row 334's page).
// - Choice: drop `ESC [` sequences up to their final byte (colours among
//   them), any other `ESC` pair, and control characters except a tab, by a
//   scan rather than a regular expression (lint refuses control characters
//   in patterns).

import type { NodeLog } from "../wire/proto";
import type { Outcome } from "./outcome";

/** What "Node logs" shows below its path line. Paseo opens its modal only
 * once the logs have arrived, so it never shows a failure; the page opens
 * the dialog first, so a failed call shows its error instead of the
 * contents, and "(log file is empty)" stands only for a file with no lines
 * (the coordinator's ruling on the node reviewer's note, row 334). */
export type LogContents =
  | { kind: "lines"; text: string }
  | { kind: "empty" }
  | { kind: "error"; message: string }
  /** The node keeps no log file: its path line says where it writes. */
  | { kind: "none" };

/** The contents for a `node.log` call's outcome. */
export function logContents(log: Outcome<NodeLog>): LogContents {
  if (!log.ok) return { kind: "error", message: log.problem.message };
  if (log.value.state !== "file") return { kind: "none" };
  return log.value.lines.length === 0
    ? { kind: "empty" }
    : { kind: "lines", text: logText(log.value.lines) };
}

/** `line` without a terminal's escape sequences or control characters
 * (a tab is kept). */
export function readableLine(line: string): string {
  let shown = "";
  let index = 0;
  while (index < line.length) {
    const code = line.charCodeAt(index);
    index += 1;
    if (code === 0x1b) {
      if (line.charCodeAt(index) === 0x5b) {
        index += 1;
        // Parameters and intermediates, up to one final byte.
        while (index < line.length) {
          const part = line.charCodeAt(index);
          index += 1;
          if (part >= 0x40 && part <= 0x7e) break;
        }
      } else {
        index += 1;
      }
    } else if (code === 0x09 || !isControl(code)) {
      shown += line.charAt(index - 1);
    }
  }
  return shown;
}

/** The lines as one text, one per line. */
export function logText(lines: readonly string[]): string {
  return lines.map(readableLine).join("\n");
}

function isControl(code: number): boolean {
  return code < 0x20 || (code >= 0x7f && code <= 0x9f);
}
