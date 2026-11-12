// Find in a session (DESIGN.md §6.16): agent-orchestrator's find bar, at
// the conversation's top right: a search field with the count inside it,
// then previous, next and close. Enter goes to the next match, Shift+Enter
// to the previous, Escape closes and gives focus back; matching wraps and
// says so. What matches is `data/find.ts`'s, over the session's model;
// putting a match on screen is the timeline's (`reveal.ts`).

import "./find-bar.css";

import { useEffect, useRef, useState } from "react";

import {
  countSpoken,
  countText,
  findInSlices,
  type Match,
  MAX_QUERY,
  step,
} from "../../data/find";
import type { Entry } from "../../data/timeline-model";
import { IconButton, TextField, Tooltip } from "../../ui/kit";

/** How long typing settles before a search (GOAT's 100 ms). */
const SETTLE_MS = 100;

/** How long a growing conversation settles before the search is run
 * again (GOAT's live refresh). */
const REFRESH_MS = 300;

export function FindBar({
  entries,
  onReveal,
  onClose,
  focusRequest,
}: {
  entries: readonly Entry[];
  /** The current match changed (`null`: none). */
  onReveal: (match: Match | null, query: string) => void;
  onClose: () => void;
  /** Changes when the find shortcut is pressed again: focus and select. */
  focusRequest: number;
}) {
  const [query, setQuery] = useState("");
  // The matches and the query they were found for: the field can be
  // ahead of them while a search runs.
  const [found, setFound] = useState<{
    query: string;
    matches: readonly Match[];
  }>({ query: "", matches: [] });
  const matches = found.matches;
  const [index, setIndex] = useState(0);
  // Every press of Enter, previous or next: the same match asked for again
  // is gone to again (the reader may have scrolled away from it).
  const [asked, setAsked] = useState(0);
  const [searching, setSearching] = useState(false);
  const [said, setSaid] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const before = useRef<Element | null>(null);
  const searched = useRef<{ query: string; entries: readonly Entry[] }>({
    query: "",
    entries: [],
  });

  useEffect(() => {
    before.current ??= document.activeElement;
    input.current?.focus();
    input.current?.select();
  }, [focusRequest]);

  useEffect(() => {
    let cancelled = false;
    const sameQuery = searched.current.query === query;
    const wait = sameQuery ? REFRESH_MS : SETTLE_MS;
    const timer = setTimeout(() => {
      setSearching(true);
      void findInSlices(entries, query, () => cancelled).then((result) => {
        if (result === null || cancelled) {
          return;
        }
        searched.current = { query, entries };
        setSearching(false);
        setFound({ query, matches: result });
        // A new query starts at its first match; a refresh of the same
        // query keeps the place, clamped.
        setIndex((current) =>
          sameQuery ? Math.min(current, Math.max(0, result.length - 1)) : 0,
        );
      });
    }, wait);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [entries, query]);

  // The match to show, by what it is rather than by its object: a search
  // run again as the session grows (REFRESH_MS) finds the same match as a
  // new object, and must not take the reader back to it each time.
  const key = matches[index]?.key ?? null;
  const nth = matches[index]?.nth ?? 0;
  useEffect(() => {
    onReveal(key === null ? null : { key, nth }, found.query);
  }, [key, nth, found.query, asked, onReveal]);

  const close = () => {
    onClose();
    if (before.current instanceof HTMLElement) {
      before.current.focus();
    }
  };
  const move = (by: 1 | -1) => {
    const next = step(matches.length, index, by);
    setIndex(next.index);
    setAsked((count) => count + 1);
    setSaid(
      next.wrapped
        ? by === 1
          ? "Wrapped to the first match."
          : "Wrapped to the last match."
        : "",
    );
  };
  const none = query.trim() !== "" && !searching && matches.length === 0;
  return (
    <div className="find-bar" role="search" aria-label="Find in session">
      <TextField
        look="popup"
        size="sm"
        icon="search"
        aria-label="Find in session"
        placeholder="Find in session"
        value={query}
        onChange={(text) => {
          setQuery(text.slice(0, MAX_QUERY));
        }}
        inputRef={input}
        onKeyDown={(event) => {
          if (event.nativeEvent.isComposing) {
            return;
          }
          if (event.key === "Enter") {
            event.preventDefault();
            move(event.shiftKey ? -1 : 1);
          } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            close();
          }
        }}
        end={
          <span className="find-bar-end">
            <span
              className={none ? "find-count none" : "find-count"}
              aria-hidden="true"
            >
              {query.trim() === ""
                ? ""
                : searching && matches.length === 0
                  ? "…"
                  : countText(matches.length, index)}
            </span>
            <span className="find-divider" aria-hidden="true" />
            <Tooltip label="Previous match">
              <IconButton
                label="Previous match"
                icon="chevron-up"
                size="xs"
                disabled={matches.length === 0}
                onClick={() => {
                  move(-1);
                }}
              />
            </Tooltip>
            <Tooltip label="Next match">
              <IconButton
                label="Next match"
                icon="chevron-down"
                size="xs"
                disabled={matches.length === 0}
                onClick={() => {
                  move(1);
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
      <span className="visually-hidden" aria-live="polite">
        {query.trim() === "" ? "" : countSpoken(matches.length, index)}
        {said === "" ? "" : ` ${said}`}
      </span>
    </div>
  );
}
