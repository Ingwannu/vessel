// When find searches (find-bar.tsx): `SETTLE_MS` after the query was
// typed, and while the session grows, again at most every `REFRESH_MS`.
//
// [Decision Log] (milestone 11, the integration's review)
// - Purpose: the find bar searched from one effect on the query and the
//   entries, which cleared its timer and cancelled the running search on
//   every change of either. A streaming session replaces its entries once
//   a batch, a few times a second, so while an agent wrote, a query typed
//   was never searched at all (the bar said "No results") and a refresh
//   over 20,000 entries never finished.
// - Choice: the query debounces (a new query cancels the old search: its
//   answer is no longer wanted); the entries throttle (a refresh is
//   scheduled once and runs on the entries of that moment; entries that
//   arrive while a search runs ask for one more refresh after it, never
//   cancel it).
// - Cost: a match that arrives mid-search is counted by the next refresh,
//   at most `REFRESH_MS` plus a search later.

import { useEffect, useRef, useState } from "react";

import { findInSlices, type Match } from "../../data/find";
import type { Entry } from "../../data/timeline-model";

/** How long typing settles before a search (GOAT's 100 ms). */
const SETTLE_MS = 100;

/** How often, at most, a growing conversation is searched again (GOAT's
 * live refresh). */
const REFRESH_MS = 300;

/** The matches and the query they were found for: the field can be
 * ahead of them while a search runs. */
export interface Searched {
  query: string;
  matches: readonly Match[];
}

/** What is being searched, held across renders. */
interface Run {
  /** The search wanted now, by number: an older one's answer is
   * dropped. */
  ticket: number;
  running: boolean;
  /** Entries arrived while a search ran: refresh once it ends. */
  dirty: boolean;
  timer: ReturnType<typeof setTimeout> | null;
}

/** Search `entries` for `query`, as find-bar.tsx's Decision Log says. */
export function useSearch(
  entries: readonly Entry[],
  query: string,
): { searched: Searched; searching: boolean } {
  const [searched, setSearched] = useState<Searched>({
    query: "",
    matches: [],
  });
  const [searching, setSearching] = useState(false);
  const latest = useRef({ entries, query });
  const run = useRef<Run>({
    ticket: 0,
    running: false,
    dirty: false,
    timer: null,
  });
  // The search itself, on the entries and query of the moment it starts.
  const search = useRef<() => void>(() => undefined);
  useEffect(() => {
    latest.current = { entries, query };
  });
  useEffect(() => {
    const state = run.current;
    const start = (): void => {
      state.timer = null;
      state.ticket += 1;
      const ticket = state.ticket;
      const { entries: now, query: asked } = latest.current;
      state.running = true;
      state.dirty = false;
      setSearching(true);
      void findInSlices(now, asked, () => ticket !== state.ticket).then(
        (result) => {
          if (result === null || ticket !== state.ticket) {
            return;
          }
          state.running = false;
          setSearching(false);
          setSearched({ query: asked, matches: result });
          if (state.dirty) {
            state.timer = setTimeout(start, REFRESH_MS);
          }
        },
      );
    };
    search.current = start;
    return () => {
      // Unmounted: nothing more is wanted.
      state.ticket += 1;
      if (state.timer !== null) {
        clearTimeout(state.timer);
      }
    };
  }, []);
  // A new query: the old search's answer is no longer wanted.
  useEffect(() => {
    const state = run.current;
    state.ticket += 1;
    state.running = false;
    if (state.timer !== null) {
      clearTimeout(state.timer);
    }
    state.timer = setTimeout(() => {
      search.current();
    }, SETTLE_MS);
  }, [query]);
  // More entries: one refresh, unless one is waiting or a search runs.
  useEffect(() => {
    const state = run.current;
    if (state.running) {
      state.dirty = true;
      return;
    }
    state.timer ??= setTimeout(() => {
      search.current();
    }, REFRESH_MS);
  }, [entries]);
  return { searched, searching };
}
