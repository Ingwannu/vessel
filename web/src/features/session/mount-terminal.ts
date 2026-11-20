// Shared xterm surface so session and standalone PTYs agree on fonts,
// input ordering, hyperlink policy and focus-sensitive resize.

import "@xterm/xterm/css/xterm.css";
import "./terminal-view.css";

import { FitAddon } from "@xterm/addon-fit";
import { type ITheme, Terminal } from "@xterm/xterm";
import { userInput } from "../../data/terminal-input";
import { openExternal } from "../../data/links";
import { prefs } from "../../data/prefs";
import { cssColor, onSchemeChange } from "../../data/theme";
import { TerminalSender } from "../../data/terminal-sender";
import type { TerminalInput, TerminalSize } from "../../wire/proto";

/** The mono stack (tokens.css `--font-mono`): Geist Mono, the real
 * monospace faces, then Pretendard, then `monospace` (the operator's font
 * rule). xterm.js measures its cell from it, so it is written out. */
const TERMINAL_FONT =
  '"Geist Mono Variable", Consolas, "SF Mono", Menlo, "JetBrains Mono", "Cascadia Mono", "DejaVu Sans Mono", "Liberation Mono", "Noto Sans Mono", Pretendard, monospace';

/** xterm's theme from the terminal's own tokens, resolved for the scheme
 * the page shows now; not the page's accent, which is black in the light
 * theme while the terminal is black in both. */
function terminalTheme(): ITheme {
  return {
    background: cssColor("--term-bg"),
    foreground: cssColor("--term-fg"),
    cursor: cssColor("--term-cursor"),
    selectionBackground: cssColor("--term-selection"),
  };
}

interface MountedTerminal {
  write: (data: Uint8Array) => void;
  close: () => void;
}

export function mountTerminal(
  element: HTMLDivElement,
  send: (input: TerminalInput) => Promise<unknown>,
  onSize: (size: TerminalSize) => void,
  onDropped: () => void,
  resize: "after-typing" | "on-attach" = "after-typing",
): MountedTerminal {
  const terminal = new Terminal({
    fontFamily: TERMINAL_FONT,
    // Appearance's code font, a pixel up, as the terminal token is.
    fontSize: prefs.get().values.appearance.code_size + 1,
    lineHeight: 1.25,
    cursorBlink: true,
    cursorStyle: "block",
    convertEol: false,
    scrollback: 5000,
    // OSC 8 hyperlinks from the session follow the timeline's link
    // policy (data/links.ts): http(s) only, after a confirm showing the
    // URL, in a new isolated tab; the UI tab is never navigated.
    linkHandler: {
      activate: (_event, uri) => {
        openExternal(uri);
      },
      allowNonHttpProtocols: false,
    },
    theme: terminalTheme(),
  });
  // The terminal's tokens differ a shade between the schemes.
  const stopScheme = onSchemeChange(() => {
    terminal.options.theme = terminalTheme();
  });
  const fit = new FitAddon();
  let codeSize = prefs.get().values.appearance.code_size;
  const stopSize = prefs.subscribe(() => {
    const next = prefs.get().values.appearance.code_size;
    if (next !== codeSize) {
      codeSize = next;
      terminal.options.fontSize = next + 1;
      fit.fit();
    }
  });
  terminal.loadAddon(fit);
  terminal.open(element);
  fit.fit();
  onSize({ cols: terminal.cols, rows: terminal.rows });
  const sender = new TerminalSender(send, onDropped);
  let typed = resize === "on-attach";
  if (resize === "on-attach") {
    sender.resize(terminal.cols, terminal.rows);
  }
  const sendSize = () => {
    if (!typed || !element.contains(document.activeElement)) {
      return;
    }
    sender.resize(terminal.cols, terminal.rows);
  };
  if (resize === "on-attach") {
    element.addEventListener("focusin", sendSize);
  }
  const input = userInput(terminal, (data) => {
    const first = !typed;
    typed = true;
    if (first) {
      sendSize();
    }
    sender.keys(data);
  });
  const observer = new ResizeObserver(() => {
    fit.fit();
    onSize({ cols: terminal.cols, rows: terminal.rows });
    sendSize();
  });
  observer.observe(element);
  return {
    write: (data: Uint8Array) => {
      terminal.write(data);
    },
    close: () => {
      stopScheme();
      stopSize();
      observer.disconnect();
      element.removeEventListener("focusin", sendSize);
      input.dispose();
      sender.close();
      terminal.dispose();
    },
  };
}
