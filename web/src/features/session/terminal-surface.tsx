import "./find-bar.css";
import "./terminal-search.css";

import {
  type RefObject,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import { MAX_QUERY } from "../../data/find";
import { currentPlatform } from "../../data/shortcuts/platform";
import { IconButton, TextField, Tooltip } from "../../ui/kit";
import { recording } from "../shortcuts/handlers";
import { TERMINAL_MATCH_LIMIT, type TerminalSearch } from "./terminal-search";

interface FindState {
  terminal: TerminalSearch;
  query: string;
  focusRequest: number;
  open: boolean;
}

/** A single xterm host and the controls that share its keyboard focus. */
export function TerminalSurface({
  host,
  label,
  find,
  compact,
}: {
  host: RefObject<HTMLDivElement | null>;
  label: string;
  find: TerminalSearch | null;
  compact: boolean;
}) {
  const [stored, setStored] = useState<FindState | null>(null);
  const state = stored?.terminal === find ? stored : null;
  const open = () => {
    if (find === null) return;
    const query = state?.query ?? "";
    if (state?.open !== true && query !== "") find.search(query);
    setStored({
      terminal: find,
      query,
      focusRequest: (state?.focusRequest ?? 0) + 1,
      open: true,
    });
  };

  return (
    <div
      className="terminal-surface"
      onKeyDownCapture={(event) => {
        if (
          find === null ||
          event.defaultPrevented ||
          event.nativeEvent.isComposing ||
          recording.get()
        )
          return;
        // Capture before xterm converts Ctrl+F to a byte for the PTY.
        // All other chords keep the terminal's ordinary key handling.
        const primary =
          currentPlatform() === "mac"
            ? event.metaKey && !event.ctrlKey
            : event.ctrlKey && !event.metaKey;
        if (
          primary &&
          !event.altKey &&
          !event.shiftKey &&
          event.key.toLowerCase() === "f"
        ) {
          event.preventDefault();
          event.stopPropagation();
          open();
        }
      }}
    >
      <div className="xterm-host" ref={host} aria-label={label} />
      {find !== null && state?.open === true ? (
        <TerminalFindBar
          find={find}
          query={state.query}
          focusRequest={state.focusRequest}
          change={(query) => {
            setStored({ ...state, query });
            find.search(query);
          }}
          close={() => {
            find.clear();
            setStored({ ...state, open: false });
            find.focus();
          }}
        />
      ) : find !== null && compact ? (
        <div className="terminal-find-launch">
          <IconButton
            label="Find in terminal"
            icon="search"
            size="sm"
            onClick={open}
          />
        </div>
      ) : null}
    </div>
  );
}

function TerminalFindBar({
  find,
  query,
  focusRequest,
  change,
  close,
}: {
  find: TerminalSearch;
  query: string;
  focusRequest: number;
  change: (query: string) => void;
  close: () => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  const { resultCount, resultIndex, bottom } = useSyncExternalStore(
    find.subscribe,
    find.get,
  );
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, [focusRequest]);
  useEffect(() => {
    const element = bar.current;
    if (element === null) return;
    const measure = () => {
      const { width, height } = element.getBoundingClientRect();
      find.measure(width, height);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [find]);

  const none = resultCount === 0;
  const total = `${String(resultCount)}${resultCount >= TERMINAL_MATCH_LIMIT ? "+" : ""}`;
  const count = none
    ? "No matches"
    : resultIndex < 0
      ? total
      : `${String(resultIndex + 1)} of ${total}`;
  return (
    <div
      ref={bar}
      className="find-bar terminal-find-bar"
      data-placement={bottom ? "bottom" : "top"}
      role="search"
      aria-label="Find in terminal"
    >
      <TextField
        look="popup"
        size="sm"
        icon="search"
        aria-label="Find in terminal"
        placeholder="Find in terminal"
        value={query}
        onChange={(text) => {
          change(text.slice(0, MAX_QUERY));
        }}
        inputRef={input}
        onKeyDown={(event) => {
          if (event.nativeEvent.isComposing) return;
          if (event.key === "Enter" || event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            if (event.key === "Escape") close();
            else find.search(query, event.shiftKey ? "previous" : "next");
          }
        }}
        end={
          <span className="find-bar-end">
            <span
              className={none ? "find-count none" : "find-count"}
              role="status"
              aria-live="polite"
            >
              {query === "" ? "" : count}
            </span>
            <span className="find-divider" aria-hidden="true" />
            <Tooltip label="Previous match">
              <IconButton
                label="Previous match"
                icon="chevron-up"
                size="xs"
                disabled={none}
                onClick={() => {
                  find.search(query, "previous");
                }}
              />
            </Tooltip>
            <Tooltip label="Next match">
              <IconButton
                label="Next match"
                icon="chevron-down"
                size="xs"
                disabled={none}
                onClick={() => {
                  find.search(query, "next");
                }}
              />
            </Tooltip>
            <Tooltip label="Close find">
              <IconButton
                label="Close find"
                icon="close"
                size="xs"
                onClick={close}
              />
            </Tooltip>
          </span>
        }
      />
    </div>
  );
}
