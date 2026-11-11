// A structured session's timeline, following its log from the start
// (data/timeline.ts folds it). Records are folded in once per frame,
// however fast they come.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { followSession } from "../../data/calls";
import { EMPTY, reduce, type Timeline } from "../../data/timeline";
import type { StreamState } from "../../wire/bridge";
import type { EventRecord } from "../../wire/proto";

export function useTimeline(
  node: string,
  session: string,
): { timeline: Timeline; stream: StreamState | null } {
  const fleet = useFleet();
  const [timeline, setTimeline] = useState<Timeline>(EMPTY);
  const [stream, setStream] = useState<StreamState | null>(null);
  useEffect(() => {
    let batch: EventRecord[] = [];
    let frame: number | null = null;
    const flush = () => {
      frame = null;
      const records = batch;
      batch = [];
      setTimeline((current) => reduce(current, records));
    };
    const stop = followSession(fleet, node, session, 0, {
      record: (record) => {
        batch.push(record);
        frame ??= window.requestAnimationFrame(flush);
      },
      state: setStream,
    });
    return () => {
      stop();
      if (frame !== null) {
        window.cancelAnimationFrame(frame);
      }
    };
  }, [fleet, node, session]);
  return { timeline, stream };
}
