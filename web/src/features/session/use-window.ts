// The timeline's window (data/window.ts): which rows are drawn as the log
// scrolls, their heights as measured, the log kept at its end while the
// reader is there, and a row gone to by key held where it was put.
//
// [Decision Log] (milestone 11, track P; docs/PERFORMANCE.md)
// - Purpose: a 20,000-entry timeline put 180,000 elements in the page;
//   each scroll step took 266 ms and the heap held 108 MiB. Only the
//   entries near the viewport need to exist.
// - Constraint: entries differ in height (a paragraph, a tool card that
//   opens) and are known only once drawn; the log follows its end; a
//   card the reader opened stays open when it comes back into view.
// - Alternatives: a list library (@legendapp/list, as GOAT and T3 Code
//   use; react-virtual) for one list; `content-visibility: auto` (keeps
//   every element and React's work for each, saves only layout and
//   paint).
// - Choice: spacers before and after the drawn entries, sized from the
//   heights measured (one ResizeObserver over the drawn rows,
//   window-row.tsx) and their mean for the rest. The browser's scroll
//   anchoring keeps the view still when a measurement above it changes
//   a spacer. A `<details>` the reader toggles is remembered by its entry
//   and restored when its row is drawn again.
// - Cost: entries outside the window are not in the document: the
//   browser's find-in-page and a screen reader's browse mode reach only
//   what is drawn, and new entries are announced from a live region of
//   their text (timeline.tsx) instead of from the list itself.
//
// With track W (merged in milestone 11): the rows are data/rows.ts's (a
// closed run of tool calls is one row); the end is followed only while
// the reader is within `NEAR_END` of it (a reader who scrolled up, or was
// taken up by find or the outline, is not pulled down by new records, as
// agent-orchestrator's find pauses "follow latest"); and `target` goes to
// a row by key, drawn or not, and holds it there (window-pin.ts says why
// and how).

import {
  type RefObject,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { covers, offsets, scrollFor, type Span, span } from "../../data/window";
import {
  hold,
  markPinned,
  ownScroll,
  type Pin,
  TAKE_BACK,
  type Target,
} from "./window-pin";

/** Pixels drawn beyond each edge of the viewport. */
const MARGIN = 1200;
/** A first guess at an entry's height, before any is measured. */
const FALLBACK = 72;
/** How close to the end counts as reading the end, in pixels. */
const NEAR_END = 48;
/** How many times in one frame a held pin may have the window worked out
 * again before paint (each pass draws the rows its correction brought
 * into view; the next frame's scroll finishes any it did not). */
const PASSES = 4;

/** What the drawn rows share: the observer measuring them and the open
 * state of the `<details>` the reader toggled, by entry. */
export interface Rows {
  observer: ResizeObserver;
  opened: Map<string, readonly boolean[]>;
}

export interface Window {
  /** The ref of the element that scrolls. */
  scroller: RefObject<HTMLDivElement | null>;
  span: Span;
  rows: Rows;
}

/** Where the reader is: following the end, holding a pin, and how many
 * passes this frame's pin has had. */
interface Reading {
  /** Whether this list follows its end at all (`useWindow`'s `follow`). */
  follows: boolean;
  following: boolean;
  pin: Pin | null;
  passes: number;
  /** The last target taken, by `seq`. */
  taken: number | null;
}

/** The window over `items`, by their keys, going to `target` when it
 * changes. A list that is read from its top (the code panel's diffs and
 * files, milestone 11's track B-page) passes `follow: false`; the
 * timeline follows its end. */
export function useWindow(
  items: readonly { key: string }[],
  target: Target | null,
  { follow = true }: { follow?: boolean } = {},
): Window {
  const scroller = useRef<HTMLDivElement>(null);
  const [heights, setHeights] = useState<ReadonlyMap<string, number>>(
    () => new Map(),
  );
  const [view, setView] = useState({ top: 0, height: 0 });
  const [rows] = useState<Rows>(() => ({
    observer: new ResizeObserver((changes) => {
      setHeights((known) => measure(known, changes));
    }),
    opened: new Map(),
  }));
  const reading = useRef<Reading>({
    follows: follow,
    following: follow,
    pin: null,
    passes: 0,
    taken: null,
  });
  useLayoutEffect(
    () => () => {
      rows.observer.disconnect();
    },
    [rows],
  );
  useLayoutEffect(() => {
    const element = scroller.current;
    if (element === null) {
      return undefined;
    }
    return watch(element, rows, reading.current, setView);
  }, [rows]);
  const starts = useMemo(
    () =>
      offsets(
        items.map((item) => item.key),
        heights,
        FALLBACK,
      ),
    [items, heights],
  );
  const drawn = useMemo(
    () => span(starts, view.top, view.height, MARGIN),
    [starts, view],
  );
  // A new target: scroll to where its row is estimated to be, so it is
  // drawn in the render this asks for, and hold it.
  useLayoutEffect(() => {
    const element = scroller.current;
    const state = reading.current;
    if (target === null || element === null || state.taken === target.seq) {
      return;
    }
    state.taken = target.seq;
    const index = items.findIndex((item) => item.key === target.key);
    if (index < 0) {
      return;
    }
    state.following = false;
    state.passes = 0;
    markPinned(element, true);
    element.scrollTop = scrollFor(
      starts,
      index,
      element.clientHeight,
      target.align,
    );
    state.pin = { target, anchor: null, at: element.scrollTop };
    setView(viewOf(element));
  }, [target, items, starts]);
  // After every change of the rows, their heights or the view: a held pin
  // corrected where the change moved it, or the end kept while the reader
  // is there.
  const end = useRef({ count: -1, height: -1 });
  useLayoutEffect(() => {
    const element = scroller.current;
    const state = reading.current;
    if (element === null) {
      return;
    }
    if (state.pin !== null) {
      const moved = hold(element, state.pin);
      const top = element.scrollTop;
      if (
        moved &&
        state.passes < PASSES &&
        !covers(starts, drawn, top, element.clientHeight)
      ) {
        state.passes += 1;
        setView(viewOf(element));
      }
      return;
    }
    const grew = items.length !== end.current.count;
    if (
      state.following &&
      (grew || element.scrollHeight !== end.current.height)
    ) {
      element.scrollTop = element.scrollHeight;
    }
    end.current = { count: items.length, height: element.scrollHeight };
  }, [items, starts, drawn]);
  return { scroller, span: drawn, rows };
}

/** `element`'s view as the window reads it. */
function viewOf(element: HTMLElement): { top: number; height: number } {
  return { top: element.scrollTop, height: element.clientHeight };
}

/** `known` with the heights `changes` measured; the same map when none
 * changed, so nothing draws again. */
function measure(
  known: ReadonlyMap<string, number>,
  changes: readonly ResizeObserverEntry[],
): ReadonlyMap<string, number> {
  let next: Map<string, number> | null = null;
  for (const change of changes) {
    const key = change.target.getAttribute("data-key");
    const size = change.borderBoxSize[0]?.blockSize;
    if (key !== null && size !== undefined && known.get(key) !== size) {
      next ??= new Map(known);
      next.set(key, size);
    }
  }
  return next ?? known;
}

/** Follow `element`'s scrolling and size into `setView`, once a frame;
 * keep `reading` told whether the reader is at the end and let its pin go
 * when the reader takes the scroll back; and remember the `<details>`
 * toggled in its rows. Returns the undo. */
function watch(
  element: HTMLElement,
  rows: Rows,
  reading: Reading,
  setView: (view: { top: number; height: number }) => void,
): () => void {
  let frame: number | null = null;
  const read = () => {
    frame = null;
    reading.passes = 0;
    setView(viewOf(element));
  };
  const takeBack = () => {
    if (reading.pin !== null) {
      reading.pin = null;
      markPinned(element, false);
    }
  };
  const onScroll = () => {
    if (reading.pin !== null && !ownScroll(element, reading.pin)) {
      takeBack();
    }
    if (reading.pin === null && reading.follows) {
      reading.following =
        element.scrollHeight - element.scrollTop - element.clientHeight <
        NEAR_END;
    }
    frame ??= window.requestAnimationFrame(read);
  };
  const onToggle = (event: Event) => {
    const row =
      event.target instanceof Element
        ? event.target.closest("[data-key]")
        : null;
    const key = row?.getAttribute("data-key") ?? null;
    if (row !== null && key !== null) {
      rows.opened.set(
        key,
        Array.from(row.querySelectorAll("details"), (details) => details.open),
      );
    }
  };
  const resized = new ResizeObserver(onScroll);
  resized.observe(element);
  read();
  element.addEventListener("scroll", onScroll, { passive: true });
  for (const name of TAKE_BACK) {
    element.addEventListener(name, takeBack, { passive: true });
  }
  // `toggle` does not bubble; it is seen on its way down.
  element.addEventListener("toggle", onToggle, { capture: true });
  return () => {
    element.removeEventListener("scroll", onScroll);
    for (const name of TAKE_BACK) {
      element.removeEventListener(name, takeBack);
    }
    element.removeEventListener("toggle", onToggle, { capture: true });
    resized.disconnect();
    if (frame !== null) {
      window.cancelAnimationFrame(frame);
    }
  };
}
