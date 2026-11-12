// The timeline's column (agent-orchestrator's transcript: at most 768px,
// centred, or Appearance's content width), following the log as it grows.
// Only the entries near the view are drawn (use-window.ts); the space of
// the others is kept. New entries are announced politely from a live
// region of their text, once for the list, not for every row drawn while
// scrolling.
//
// Its one seam for find and the outline is `reveal` (reveal.ts): an
// entry to bring into view by key.

import "./timeline.css";

import { useEffect, useLayoutEffect, useRef } from "react";

import { spoken } from "../../data/timeline-text";
import type { Timeline as Log } from "../../data/timeline-model";
import { TimelineEntry } from "./folds";
import { type Reveal, revealEntry } from "./reveal";
import { useWindow } from "./use-window";
import { WindowRow } from "./window-row";

export function Timeline({
  timeline,
  reveal = null,
  scrollerRef,
}: {
  timeline: Log;
  reveal?: Reveal | null;
  /** The scrolling element, for the outline. */
  scrollerRef?: (element: HTMLDivElement | null) => void;
}) {
  const entries = timeline.entries;
  const { scroller, span, rows } = useWindow(entries);
  const column = useRef<HTMLElement>(null);
  const last = entries.at(-1);
  useEffect(() => {
    if (reveal !== null && column.current !== null) {
      revealEntry(column.current, reveal);
    }
  }, [reveal]);
  useLayoutEffect(() => {
    scrollerRef?.(scroller.current);
    return () => {
      scrollerRef?.(null);
    };
  }, [scroller, scrollerRef]);
  return (
    <div className="timeline" ref={scroller}>
      {/* A named region: the entries are found here, apart from the live
          region below, which repeats the newest one's text. */}
      <section className="timeline-column" aria-label="Timeline" ref={column}>
        <div className="t-space" style={{ height: span.before }} />
        {entries.slice(span.start, span.end).map((entry) => (
          <WindowRow
            key={entry.key}
            id={entry.key}
            last={entry === last}
            rows={rows}
          >
            <TimelineEntry entry={entry} />
          </WindowRow>
        ))}
        <div className="t-space" style={{ height: span.after }} />
        {last === undefined ? (
          <p className="t-meta">Waiting for the session&apos;s first record…</p>
        ) : null}
      </section>
      <p className="visually-hidden" aria-live="polite">
        {last === undefined ? "" : spoken(last)}
      </p>
    </div>
  );
}
