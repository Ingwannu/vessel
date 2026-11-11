// The timeline's column (agent-orchestrator's transcript: at most 768px,
// centred), following the log as it grows. Only the entries near the view
// are drawn (use-window.ts); the space of the others is kept. New entries
// are announced politely from a live region of their text, once for the
// list, not for every row drawn while scrolling.

import "./timeline.css";

import { spoken } from "../../data/timeline-text";
import type { Timeline as Log } from "../../data/timeline-model";
import { TimelineItem } from "./timeline-item";
import { useWindow } from "./use-window";
import { WindowRow } from "./window-row";

export function Timeline({ timeline }: { timeline: Log }) {
  const entries = timeline.entries;
  const { scroller, span, rows } = useWindow(entries);
  const last = entries.at(-1);
  return (
    <div className="timeline" ref={scroller}>
      {/* A named region: the entries are found here, apart from the live
          region below, which repeats the newest one's text. */}
      <section className="timeline-column" aria-label="Timeline">
        <div className="t-space" style={{ height: span.before }} />
        {entries.slice(span.start, span.end).map((entry) => (
          <WindowRow
            key={entry.key}
            id={entry.key}
            last={entry === last}
            rows={rows}
          >
            <TimelineItem entry={entry} />
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
