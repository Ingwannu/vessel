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
