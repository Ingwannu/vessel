// xterm already owns the wrapped buffer, alternate screen and cell
// coordinates. Find asks that same buffer instead of replaying PTY bytes.
import {
  type ISearchResultChangeEvent,
  SearchAddon,
} from "@xterm/addon-search";
import type { IMarker, Terminal } from "@xterm/xterm";
import { Store } from "../../data/store";

export const TERMINAL_MATCH_LIMIT = 20_000;
type Direction = "next" | "previous";
const EMPTY = { resultIndex: -1, resultCount: 0, bottom: false };

export interface TerminalSearch {
  get: () => ISearchResultChangeEvent & { bottom: boolean };
  subscribe: (listener: () => void) => () => void;
  focus: () => void;
  measure: (width: number, height: number) => void;
  clear: () => void;
  search: (query: string, direction?: Direction) => void;
  write: (data: Uint8Array, afterWrite?: () => void) => void;
}

export function terminalSearch(terminal: Terminal): TerminalSearch {
  const results = new Store(EMPTY);
  let addon: SearchAddon | null = null;
  let latest: ISearchResultChangeEvent = EMPTY;
  let widget: { width: number; height: number } | null = null;
  let anchor: IMarker | undefined;
  let pendingWrites = 0;

  const releaseViewport = () => {
    anchor?.dispose();
    anchor = undefined;
  };

  terminal.onWriteParsed(() => {
    // A search turns even the bottom into an inspected position. The
    // marker follows the buffer if new output trims its scrollback.
    if (anchor !== undefined && !anchor.isDisposed) {
      terminal.scrollToLine(anchor.line);
    }
    // A parser turn may yield with another submitted write still queued.
    if (pendingWrites === 0) releaseViewport();
  });

  const publish = () => {
    const next = {
      ...latest,
      bottom:
        widget !== null && latest.resultIndex >= 0 && covers(terminal, widget),
    };
    const current = results.get();
    if (
      next.resultIndex !== current.resultIndex ||
      next.resultCount !== current.resultCount ||
      next.bottom !== current.bottom
    ) {
      results.set(next);
    }
  };
  // Scrolling or resizing can put the current match behind the bar even
  // when the count stays the same.
  terminal.onScroll(publish);
  terminal.onResize(publish);

  const clear = () => {
    releaseViewport();
    if (addon !== null) {
      // Disposal cancels the addon's refresh of the previous query on
      // live output. The terminal retains its ordinary text selection.
      addon.dispose();
      addon = null;
    }
    latest = EMPTY;
    publish();
  };

  return {
    get: results.get,
    subscribe: results.subscribe,
    focus: () => {
      terminal.focus();
    },
    measure: (width: number, height: number) => {
      widget = { width, height };
      publish();
    },
    clear,
    write(data, afterWrite) {
      if (latest.resultCount > 0 && anchor === undefined) {
        const { viewportY, baseY, cursorY } = terminal.buffer.active;
        anchor = terminal.registerMarker(viewportY - baseY - cursorY);
      }
      if (anchor === undefined) {
        terminal.write(data, afterWrite);
        return;
      }
      pendingWrites += 1;
      terminal.write(data, () => {
        pendingWrites -= 1;
        afterWrite?.();
      });
    },
    search(query: string, direction?: Direction) {
      if (query === "") {
        clear();
        return;
      }
      releaseViewport();
      if (addon === null) {
        addon = new SearchAddon({ highlightLimit: TERMINAL_MATCH_LIMIT });
        terminal.loadAddon(addon);
        addon.onDidChangeResults((result) => {
          latest = result;
          publish();
        });
      }
      const theme = terminal.options.theme;
      const match = theme?.yellow ?? "#b58900";
      const active = theme?.blue ?? "#268bd2";
      const options = {
        incremental: direction === undefined,
        decorations: {
          matchBorder: match,
          matchOverviewRuler: match,
          activeMatchBorder: active,
          activeMatchColorOverviewRuler: active,
        },
      };
      if (direction === "next") addon.findNext(query, options);
      else addon.findPrevious(query, options);
      publish();
    },
  };
}

/** The top-right bar moves down if it would cover the selected match. */
function covers(terminal: Terminal, widget: { width: number; height: number }) {
  const selection = terminal.getSelectionPosition();
  const screen = terminal.element?.querySelector(".xterm-screen");
  if (selection === undefined || screen === undefined || screen === null) {
    return false;
  }
  const { width, height } = screen.getBoundingClientRect();
  const { viewportY } = terminal.buffer.active;
  const top = (selection.start.y - viewportY) * (height / terminal.rows);
  const bottom = (selection.end.y - viewportY + 1) * (height / terminal.rows);
  const right =
    selection.start.y === selection.end.y
      ? (selection.end.x * width) / terminal.cols
      : width;
  return top < widget.height && bottom > 0 && right > width - widget.width;
}
