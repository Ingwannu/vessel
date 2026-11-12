// The timeline's column (agent-orchestrator's transcript: at most 768px,
// centred, or Appearance's content width), following the log as it grows
// while the reader is at its end. Only the rows near the view are drawn
// (use-window.ts); the space of the others is kept. New entries are
// announced politely from a live region of their text, once for the list,
// not for every row drawn while scrolling.
//
// Its one seam for find and the outline is `reveal` (reveal.ts): an entry
// to bring into view by key, which the window goes to whether or not its
// row is drawn.

import "./timeline.css";

import { useLayoutEffect, useMemo, useRef } from "react";

import { rowIndex, rowsOf } from "../../data/rows";
import { spoken } from "../../data/timeline-text";
import type { Entry, Timeline as Log } from "../../data/timeline-model";
import { TimelineEntry, useFolds } from "./folds";
import { finding, type Reveal, showReveal } from "./reveal";
import { useWindow } from "./use-window";
import type { Target } from "./window-pin";
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
  const { folds, open } = useFolds();
  const rows = useMemo(
    () => rowsOf(entries, folds, open),
    [entries, folds, open],
  );
  // The row a reveal names: the entry's own, or its run's lead when it is
  // folded away. Worked out when the rows or the reveal change, not on
  // every scroll frame.
  const revealed = useMemo(
    () =>
      reveal === null
        ? null
        : (rows[rowIndex(rows, reveal.key, folds)] ?? null),
    [rows, reveal, folds],
  );
  const rowKey = revealed?.key ?? null;
  // The row last marked for a find, and the entry it drew then: a row
  // drawn again (the reader scrolled away and back), or its entry grown
  // (a message still streaming, whose text nodes React rewrote, which
  // collapses the ranges in them), is marked again.
  const marked = useRef<{ row: Element; entry: Entry | null } | null>(null);
  const target = useMemo<Target | null>(
    () =>
      reveal === null || rowKey === null
        ? null
        : {
            key: rowKey,
            seq: reveal.seq,
            align: reveal.match === null ? "start" : "center",
            show: (row) => {
              marked.current = { row, entry: revealed };
              return showReveal(row, reveal);
            },
          },
    [reveal, rowKey, revealed],
  );
  const windowed = useWindow(rows, target);
  const { scroller, span } = windowed;
  useLayoutEffect(() => {
    scrollerRef?.(scroller.current);
    return () => {
      scrollerRef?.(null);
    };
  }, [scroller, scrollerRef]);
  const find = finding(reveal);
  useLayoutEffect(() => {
    const element = scroller.current;
    if (find === null || rowKey === null || element === null) {
      marked.current = null;
      return;
    }
    const row = element.querySelector(`[data-key="${CSS.escape(rowKey)}"]`);
    if (
      row !== null &&
      (row !== marked.current?.row || revealed !== marked.current.entry)
    ) {
      marked.current = { row, entry: revealed };
      showReveal(row, find);
    }
  }, [find, rowKey, revealed, scroller, span]);
  const last = rows.at(-1);
  const newest = entries.at(-1);
  return (
    <div className="timeline" ref={scroller}>
      {/* A named region: the entries are found here, apart from the live
          region below, which repeats the newest one's text. */}
      <section className="timeline-column" aria-label="Timeline">
        <div className="t-space" style={{ height: span.before }} />
        {rows.slice(span.start, span.end).map((entry) => (
          <WindowRow
            key={entry.key}
            id={entry.key}
            last={entry === last}
            rows={windowed.rows}
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
        {newest === undefined ? "" : spoken(newest)}
      </p>
    </div>
  );
}
