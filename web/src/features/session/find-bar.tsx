// Find in a session (DESIGN.md §6.16): agent-orchestrator's find bar, at
// the conversation's top right: a search field with the count inside it,
// then previous, next and close. Enter goes to the next match, Shift+Enter
// to the previous, Escape closes and gives focus back; matching wraps and
// says so. What matches is `data/find.ts`'s, over the session's model,
// searched when `use-search.ts` says; putting a match on screen is the
// timeline's (`reveal.ts`).

import "./find-bar.css";

import { useEffect, useRef, useState } from "react";

import {
  countSpoken,
  countText,
  type Match,
  MAX_QUERY,
  step,
} from "../../data/find";
import type { Entry } from "../../data/timeline-model";
import { IconButton, TextField, Tooltip } from "../../ui/kit";
import { useSearch } from "./use-search";

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
  const { searched, searching } = useSearch(entries, query);
  const matches = searched.matches;
  // The current match: a new query starts at its first (the field's
  // change sets 0); a refresh of the same query keeps the place, clamped.
  const [chosen, setChosen] = useState(0);
  const index = Math.min(chosen, Math.max(0, matches.length - 1));
  // Every press of Enter, previous or next: the same match asked for again
  // is gone to again (the reader may have scrolled away from it).
  const [asked, setAsked] = useState(0);
  const [said, setSaid] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const before = useRef<Element | null>(null);

  useEffect(() => {
    before.current ??= document.activeElement;
    input.current?.focus();
    input.current?.select();
  }, [focusRequest]);

  // The match to show, by what it is rather than by its object: a search
  // run again as the session grows (use-search.ts) finds the same match as a
  // new object, and must not take the reader back to it each time.
  const key = matches[index]?.key ?? null;
  const nth = matches[index]?.nth ?? 0;
  useEffect(() => {
    onReveal(key === null ? null : { key, nth }, searched.query);
  }, [key, nth, searched.query, asked, onReveal]);

  const close = () => {
    onClose();
    if (before.current instanceof HTMLElement) {
      before.current.focus();
    }
  };
  const move = (by: 1 | -1) => {
    const next = step(matches.length, index, by);
    setChosen(next.index);
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
          setChosen(0);
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
