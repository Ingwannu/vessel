// A structured session's timeline, following its log from the start
// (data/timeline.ts folds it). Records are folded in once per frame,
// however fast they come.
//
// A followed log ends with its session (the stream says `ended`). A
// session started again (protocol 10's `session.resume`, here or from
// another device) continues the same log, so once the session is live
// again the log is followed again from the record after the last one
// folded: the same timeline grows, nothing is read twice.

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { readConversation } from "../../data/conversation-calls";
import { reconcileSnapshot } from "../../data/timeline-snapshot";
import { followSession } from "../../data/calls";
import { EMPTY, type Timeline } from "../../data/timeline-model";
import { reduce } from "../../data/timeline";
import type { NodeKey, StreamState } from "../../wire/bridge";
import type { Conversation, EventRecord, SessionId } from "../../wire/proto";

export function useTimeline(
  node: NodeKey,
  session: SessionId,
  /** Whether the session runs, as the machine's list last said. */
  live = false,
): { timeline: Timeline; stream: StreamState | null } {
  const fleet = useFleet();
  const [timeline, setTimeline] = useState<Timeline>(EMPTY);
  const [stream, setStream] = useState<StreamState | null>(null);
  // The record to follow from when the log is followed again: the one
  // after the last that arrived (they arrive in order, each once).
  const next = useRef(0);
  // Each follow of the log: a new one once the session runs again after
  // its log ended.
  const ended = stream?.state === "ended";
  const [round, setRound] = useState(0);
  const [roundLive, setRoundLive] = useState(live);
  if (live !== roundLive) {
    setRoundLive(live);
    if (live && ended) {
      setRound(round + 1);
    }
  }
  useEffect(() => {
    let batch: EventRecord[] = [];
    let snapshot: Conversation | null = null;
    let disposed = false;
    const covered = (current: Timeline) =>
      snapshot === null ? current : reconcileSnapshot(current, snapshot);
    const refresh = () => {
      void readConversation(fleet, node, session).then((outcome) => {
        if (disposed || !outcome.ok || outcome.value.through == null) return;
        if ((snapshot?.through ?? -1) > outcome.value.through) return;
        snapshot = outcome.value;
        setTimeline(covered);
      });
    };
    let frame: number | null = null;
    const flush = () => {
      frame = null;
      const records = batch;
      batch = [];
      setTimeline((current) => covered(reduce(current, records)));
    };
    const stop = followSession(fleet, node, session, next.current, {
      record: (record) => {
        next.current = record.seq + 1;
        batch.push(record);
        frame ??= window.requestAnimationFrame(flush);
      },
      state: (state) => {
        setStream(state);
        if (state.state === "resumed" || state.state === "missing") refresh();
      },
    });
    refresh();
    return () => {
      disposed = true;
      stop();
      if (frame !== null) {
        window.cancelAnimationFrame(frame);
      }
    };
  }, [fleet, node, session, round]);
  return { timeline, stream };
}
