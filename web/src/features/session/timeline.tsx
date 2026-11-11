// The timeline's column (agent-orchestrator's transcript: at most 768px,
// centred, or Appearance's content width), following the log as it grows.
// New entries are announced politely, once for the list, not for every
// chunk.
//
// It follows the end only while the reader is there (within `NEAR_END`
// of it): a reader who scrolled up, or was taken up by find or the
// outline, is not pulled down by new records (agent-orchestrator's find
// pauses "follow latest" the same way).
//
// Its one seam for find and the outline is `reveal` (reveal.ts): an
// entry to bring into view by key.

import "./timeline.css";

import { useCallback, useEffect, useRef } from "react";

import type { Timeline as Log } from "../../data/timeline-model";
import { type Reveal, revealEntry } from "./reveal";
import { TimelineEntry } from "./folds";

/** How close to the end counts as reading the end, in pixels. */
const NEAR_END = 48;

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
  const end = useRef<HTMLDivElement>(null);
  const column = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  const count = timeline.entries.length;
  useEffect(() => {
    if (atEnd.current) {
      end.current?.scrollIntoView({ block: "end" });
    }
  }, [count]);
  useEffect(() => {
    if (reveal !== null && column.current !== null) {
      revealEntry(column.current, reveal);
    }
  }, [reveal]);
  const scroller = useCallback(
    (element: HTMLDivElement | null) => {
      scrollerRef?.(element);
    },
    [scrollerRef],
  );
  return (
    <div
      className="timeline"
      ref={scroller}
      onScroll={(event) => {
        const box = event.currentTarget;
        atEnd.current =
          box.scrollHeight - box.scrollTop - box.clientHeight < NEAR_END;
      }}
    >
      <div className="timeline-column" aria-live="polite" ref={column}>
        {timeline.entries.map((entry) => (
          <TimelineEntry key={entry.key} entry={entry} />
        ))}
        {count === 0 ? (
          <p className="t-meta">Waiting for the session&apos;s first record…</p>
        ) : null}
        <div ref={end} />
      </div>
    </div>
  );
}
