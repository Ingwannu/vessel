// Marking a session read while it is in front (milestone 11, Track
// A-page; ADR 0026): `read { through }`, the record after the last one
// shown, sent when the tab is visible and focused and the session has
// shown something that asks for attention since the last mark (or the
// list says unread); at most once a second; never right after the person
// marked it unread here (data/read-hold.ts); and no more to a machine
// that answered `forbidden` (a device given only `read`: the page cannot
// know beforehand, no scope reaches it).

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { markSession } from "../../data/conversation-calls";
import { hasConversation } from "../../data/delivery";
import { isUnread, readThrough } from "../../data/marks";
import { errorCode } from "../../data/outcome";
import { readHolds, takeReadHold } from "../../data/read-hold";
import { useStore } from "../../data/store";
import type { Timeline } from "../../data/timeline-model";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";

/** Machines that refused a read mark from this page. */
const readOnly = new Set<string>();

/** At most one mark a second. */
const GAP_MS = 1000;

function inFront(): boolean {
  return document.visibilityState === "visible" && document.hasFocus();
}

/** Whether the page is in front, as it changes. */
function useFront(): boolean {
  const [front, setFront] = useState(inFront);
  useEffect(() => {
    const update = () => {
      setFront(inFront());
    };
    document.addEventListener("visibilitychange", update);
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    return () => {
      document.removeEventListener("visibilitychange", update);
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
    };
  }, []);
  return front;
}

export function useReadMarks(
  node: NodeKey,
  session: SessionInfo,
  timeline: Timeline,
  protocol: number | null,
): void {
  const fleet = useFleet();
  const front = useFront();
  const sent = useRef(-1);
  const last = useRef(0);
  const heldAt = useRef<number | null>(null);
  const holds = useStore(readHolds, (held) => held);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    if (holds.has(session.id) && takeReadHold(session.id)) {
      heldAt.current = timeline.next;
    }
    if (!hasConversation(protocol) || readOnly.has(node)) {
      return undefined;
    }
    const through = readThrough({
      front,
      unread: isUnread(session),
      attention: timeline.attention,
      next: timeline.next,
      sent: sent.current,
      heldAt: heldAt.current,
    });
    if (through === null) {
      return undefined;
    }
    const wait = last.current + GAP_MS - Date.now();
    if (wait > 0) {
      const timer = window.setTimeout(() => {
        setTick((count) => count + 1);
      }, wait);
      return () => {
        window.clearTimeout(timer);
      };
    }
    sent.current = through;
    last.current = Date.now();
    heldAt.current = null;
    void markSession(fleet, node, session.id, { mark: "read", through }).then(
      (outcome) => {
        if (outcome.ok) {
          fleet.putSession(node, outcome.value);
        } else if (errorCode(outcome.problem) === "forbidden") {
          readOnly.add(node);
        }
      },
    );
    return undefined;
  }, [
    fleet,
    node,
    session,
    timeline.attention,
    timeline.next,
    protocol,
    front,
    holds,
    tick,
  ]);
}
