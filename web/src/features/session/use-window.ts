// The timeline's window (data/window.ts): which entries are drawn as the
// log scrolls, their heights as measured, and the log kept at its end
// whenever it grows, as the timeline always did.
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

import {
  type RefObject,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { offsets, type Span, span } from "../../data/window";

/** Pixels drawn beyond each edge of the viewport. */
const MARGIN = 1200;
/** A first guess at an entry's height, before any is measured. */
const FALLBACK = 72;

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

/** The window over `items`, by their keys. */
export function useWindow(items: readonly { key: string }[]): Window {
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
    return watch(element, rows, setView);
  }, [rows]);
  // The end whenever the log grows, and again whenever a measurement
  // moves it while the reader is there.
  const end = useRef({ count: -1, height: -1 });
  useLayoutEffect(() => {
    const element = scroller.current;
    if (element === null) {
      return;
    }
    const atEnd =
      element.scrollTop + element.clientHeight >= element.scrollHeight - 4;
    const grew = items.length !== end.current.count;
    if (grew || (atEnd && element.scrollHeight !== end.current.height)) {
      element.scrollTop = element.scrollHeight;
      end.current = { count: items.length, height: element.scrollHeight };
    }
  });
  const starts = useMemo(
    () =>
      offsets(
        items.map((item) => item.key),
        heights,
        FALLBACK,
      ),
    [items, heights],
  );
  return {
    scroller,
    span: span(starts, view.top, view.height, MARGIN),
    rows,
  };
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

/** Follow `element`'s scrolling and size into `setView`, once a frame,
 * and remember the `<details>` toggled in its rows; returns the undo. */
function watch(
  element: HTMLElement,
  rows: Rows,
  setView: (view: { top: number; height: number }) => void,
): () => void {
  let frame: number | null = null;
  const read = () => {
    frame = null;
    setView({ top: element.scrollTop, height: element.clientHeight });
  };
  const onScroll = () => {
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
  // `toggle` does not bubble; it is seen on its way down.
  element.addEventListener("toggle", onToggle, { capture: true });
  return () => {
    element.removeEventListener("scroll", onScroll);
    element.removeEventListener("toggle", onToggle, { capture: true });
    resized.disconnect();
    if (frame !== null) {
      window.cancelAnimationFrame(frame);
    }
  };
}
