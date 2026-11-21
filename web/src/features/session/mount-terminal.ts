// Shared xterm surface so session and standalone PTYs agree on fonts,
// input ordering, hyperlink policy and focus-sensitive resize.
//
// Terminal queries (TRACKER 826, ADR 0073 for the page): each write is
// history or live, as the view's follow says. The page's own handlers
// (`data/terminal-queries.ts`) answer only while no history write is
// still being parsed, and only when this view has the duty: it opened
// the terminal, or it has focus and the person has typed into it.
// xterm parses writes in order and calls each write's callback right
// after its own bytes, and a follow's history always comes before its
// live records, so a count of history writes not yet parsed is enough.
//
// The key bar (P4, `key-bar.tsx`, `data/terminal-keys.ts`) presses keys
// through the same path as typing, and its sticky modifiers apply to the
// next key, from the bar or typed.

import "@xterm/xterm/css/xterm.css";
import "./terminal-view.css";

import { FitAddon } from "@xterm/addon-fit";
import { type ITheme, Terminal } from "@xterm/xterm";
import { userInput } from "../../data/terminal-input";
import {
  type BarKey,
  type Modifier,
  NO_STICKY,
  pressKey,
  type Sticky,
  typedWith,
} from "../../data/terminal-keys";
import type { Written } from "../../data/terminal-live";
import {
  answerQueries,
  type Drawn,
  hasDuty,
  type Origin,
  rgbOf,
} from "../../data/terminal-queries";
import { openExternal } from "../../data/links";
import { prefs } from "../../data/prefs";
import { onSchemeChange } from "../../data/theme";
import { TerminalSender } from "../../data/terminal-sender";
import { type TerminalPalette, terminalTheme } from "./terminal-palette";
import type { TerminalInput, TerminalSize } from "../../wire/proto";
import { terminalSearch, type TerminalSearch } from "./terminal-search";

/** The mono stack (tokens.css `--font-mono`): Geist Mono, the real
 * monospace faces, then Pretendard, then `monospace` (the operator's font
 * rule). xterm.js measures its cell from it, so it is written out. */
const TERMINAL_FONT =
  '"Geist Mono Variable", Consolas, "SF Mono", Menlo, "JetBrains Mono", "Cascadia Mono", "DejaVu Sans Mono", "Liberation Mono", "Noto Sans Mono", Pretendard, monospace';

/** The key bar's hold on a mounted terminal (P4): its sticky modifiers,
 * its keys, and the soft keyboard, which shows while the terminal holds
 * the focus. `subscribe` is for `useSyncExternalStore`. */
export interface TerminalKeyBar {
  sticky: () => Sticky;
  keyboardShown: () => boolean;
  subscribe: (listener: () => void) => () => void;
  press: (key: BarKey) => void;
  toggle: (modifier: Modifier) => void;
  toggleKeyboard: () => void;
}

interface MountedTerminal {
  write: (data: Uint8Array, written: Written) => void;
  keyBar: TerminalKeyBar;
  find: TerminalSearch;
  close: () => void;
}

/** The colours `theme` draws, for the colour queries. */
function drawnOf(theme: ITheme): Drawn {
  const colour = (css: string | undefined) =>
    css === undefined ? null : rgbOf(css);
  return {
    foreground: colour(theme.foreground),
    background: colour(theme.background),
    cursor: colour(theme.cursor),
  };
}

export function mountTerminal(
  element: HTMLDivElement,
  send: (input: TerminalInput) => Promise<unknown>,
  onSize: (size: TerminalSize) => void,
  onDropped: () => void,
  resize: "after-typing" | "on-attach" = "after-typing",
  origin: Origin = "attached",
  palette: TerminalPalette = "framed",
): MountedTerminal {
  const theme = terminalTheme(palette);
  const terminal = new Terminal({
    // The pinned search addon uses xterm's decoration API for its matches.
    allowProposedApi: true,
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
    theme,
  });
  // The palette follows the scheme (a tab's changes with it, TRACKER 837).
  let drawn = drawnOf(theme);
  const stopScheme = onSchemeChange(() => {
    const next = terminalTheme(palette);
    terminal.options.theme = next;
    drawn = drawnOf(next);
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
  // `typed` starts true for a pane that claims its size on attach; the
  // duty needs real typing.
  let keyed = false;
  let historyParsing = 0;
  /** What the person typed or pressed on the bar, in order. */
  const typeIn = (data: string) => {
    const first = !typed;
    typed = true;
    keyed = true;
    if (first) {
      sendSize();
    }
    sender.keys(data);
  };
  // The key bar's modifiers (Paseo's pane, 795-830 and 1052-1083): they
  // go with the next key, from the bar or typed, then clear.
  let sticky: Sticky = NO_STICKY;
  const listeners = new Set<() => void>();
  const changed = () => {
    for (const listener of listeners) {
      listener();
    }
  };
  const setSticky = (next: Sticky) => {
    sticky = next;
    changed();
  };
  const application = () => terminal.modes.applicationCursorKeysMode;
  const input = userInput(terminal, (data) => {
    const resolved = typedWith(data, sticky, application());
    if (resolved !== null) {
      setSticky(NO_STICKY);
    }
    typeIn(resolved ?? data);
  });
  element.addEventListener("focusin", changed);
  element.addEventListener("focusout", changed);
  const keyboardShown = () => element.contains(document.activeElement);
  const keyBar: TerminalKeyBar = {
    sticky: () => sticky,
    keyboardShown,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    press: (key) => {
      const data = pressKey(key, sticky, application());
      setSticky(NO_STICKY);
      typeIn(data);
      terminal.focus();
    },
    toggle: (modifier) => {
      setSticky({ ...sticky, [modifier]: !sticky[modifier] });
      terminal.focus();
    },
    // Paseo's toggle (999-1010): the soft keyboard shows while the
    // terminal's textarea holds the focus.
    toggleKeyboard: () => {
      if (keyboardShown()) {
        terminal.blur();
      } else {
        terminal.focus();
      }
      changed();
    },
  };
  const queries = answerQueries(terminal.parser, {
    answers: () =>
      historyParsing === 0 &&
      hasDuty({
        origin,
        // A background browser tab keeps its active element.
        focused:
          document.hasFocus() && element.contains(document.activeElement),
        keyed,
      }),
    cursor: () => ({
      row: terminal.buffer.active.cursorY,
      col: terminal.buffer.active.cursorX,
      cols: terminal.cols,
    }),
    drawn: () => drawn,
    // An answer is not typing: it neither marks the view typed nor sends
    // a size, and it waits behind the keys before it.
    send: (reply) => {
      sender.answer(reply);
    },
  });
  const observer = new ResizeObserver(() => {
    fit.fit();
    onSize({ cols: terminal.cols, rows: terminal.rows });
    sendSize();
  });
  observer.observe(element);
  return {
    keyBar,
    find: terminalSearch(terminal),
    write: (data: Uint8Array, written: Written) => {
      if (written === "live") {
        terminal.write(data);
        return;
      }
      historyParsing += 1;
      terminal.write(data, () => {
        historyParsing -= 1;
      });
    },
    close: () => {
      stopScheme();
      stopSize();
      observer.disconnect();
      element.removeEventListener("focusin", sendSize);
      element.removeEventListener("focusin", changed);
      element.removeEventListener("focusout", changed);
      listeners.clear();
      input.dispose();
      queries.dispose();
      sender.close();
      terminal.dispose();
    },
  };
}
