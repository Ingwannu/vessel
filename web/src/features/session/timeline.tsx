// The timeline's column (agent-orchestrator's transcript: at most 768px,
// centred), following the log as it grows. New entries are announced
// politely, once for the list, not for every chunk.

import "./timeline.css";

import { useEffect, useRef } from "react";

import type { Timeline as Log } from "../../data/timeline-model";
import { TimelineItem } from "./timeline-item";

export function Timeline({ timeline }: { timeline: Log }) {
  const end = useRef<HTMLDivElement>(null);
  const count = timeline.entries.length;
  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [count]);
  return (
    <div className="timeline">
      <div className="timeline-column" aria-live="polite">
        {timeline.entries.map((entry) => (
          <TimelineItem key={entry.key} entry={entry} />
        ))}
        {count === 0 ? (
          <p className="t-meta">Waiting for the session&apos;s first record…</p>
        ) : null}
        <div ref={end} />
      </div>
    </div>
  );
}
