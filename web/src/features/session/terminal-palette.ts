// A terminal's xterm theme from the page's tokens: the session page's framed
// terminal, dark in both schemes, or a workspace's terminal tab in Paseo's
// palette for each scheme (TRACKER 837; tokens.css `--tab-term-*`).

import type { ITheme } from "@xterm/xterm";

import { cssColor } from "../../data/theme";

/** Which palette a terminal is drawn in: the session page's framed
 * terminal, dark in both schemes, or a workspace's terminal tab, Paseo's
 * terminal in each scheme (TRACKER 837). */
export type TerminalPalette = "framed" | "tab";

/** xterm's theme from the terminal's own tokens, resolved for the scheme
 * the page shows now; not the page's accent, which is black in the light
 * theme. */
export function terminalTheme(palette: TerminalPalette): ITheme {
  if (palette === "framed") {
    return {
      background: cssColor("--term-bg"),
      foreground: cssColor("--term-fg"),
      cursor: cssColor("--term-cursor"),
      selectionBackground: cssColor("--term-selection"),
    };
  }
  const ansi = (index: number) => cssColor(`--tab-term-ansi-${String(index)}`);
  const foreground = cssColor("--tab-term-fg");
  return {
    background: cssColor("--tab-term-bg"),
    foreground,
    cursor: cssColor("--tab-term-cursor"),
    cursorAccent: cssColor("--tab-term-cursor-accent"),
    selectionBackground: cssColor("--tab-term-selection"),
    selectionForeground: foreground,
    black: ansi(0),
    red: ansi(1),
    green: ansi(2),
    yellow: ansi(3),
    blue: ansi(4),
    magenta: ansi(5),
    cyan: ansi(6),
    white: ansi(7),
    brightBlack: ansi(8),
    brightRed: ansi(9),
    brightGreen: ansi(10),
    brightYellow: ansi(11),
    brightBlue: ansi(12),
    brightMagenta: ansi(13),
    brightCyan: ansi(14),
    brightWhite: ansi(15),
  };
}
