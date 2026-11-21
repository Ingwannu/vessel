// The Pull request pane's reads (TRACKER 430 on the page): the session's
// pull request (`pr.get`) and its activity (`pr.timeline`), read when the
// pane opens and again on Refresh, and a failed check's report
// (`pr.check`) when its "Add logs to chat" is pressed.
//
// [Decision Log]
// - Purpose: the pane draws from the node's answers only; nothing about
//   the forge is fetched by the page.
// - Choice: one read of both on open, Refresh asks the node to read the
//   forge again (`refresh: true`); a read that comes back for a session
//   no longer shown is dropped (the effect's own flag).
// - Cost: no live updates while the pane is open (Paseo polls); Refresh
//   is the way to see new activity.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet } from "../../../context";
import { attempt, type Outcome } from "../../../data/outcome";
import { toast } from "../../../ui/toasts";
import type { NodeKey } from "../../../wire/bridge";
import type {
  CheckDetails,
  PullRequestView,
  PullTimeline,
  RunId,
  SessionId,
} from "../../../wire/proto";

export interface PullReads {
  view: Outcome<PullRequestView> | null;
  timeline: Outcome<PullTimeline> | null;
  refreshing: boolean;
  /** When the reads came back (the ages and timings are as of then). */
  readAt: number;
  refresh: () => void;
  report: (run: RunId) => Promise<Outcome<CheckDetails>>;
}

export function usePull(node: NodeKey, session: SessionId): PullReads {
  const fleet = useFleet();
  const [view, setView] = useState<Outcome<PullRequestView> | null>(null);
  const [timeline, setTimeline] = useState<Outcome<PullTimeline> | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [readAt, setReadAt] = useState(0);
  const [asked, setAsked] = useState(0);
  // What is shown: a failed Refresh keeps it and says why (Paseo's
  // pane keeps its data and toasts, pane.tsx 246-247).
  const shownView = useRef<Outcome<PullRequestView> | null>(null);
  const shownTimeline = useRef<Outcome<PullTimeline> | null>(null);

  useEffect(() => {
    let live = true;
    const refresh = asked > 0;
    void Promise.all([
      attempt(() => fleet.bridge.call(node, "pr.get", { session, refresh })),
      attempt(() => fleet.bridge.call(node, "pr.timeline", { session })),
    ]).then(([read, activity]) => {
      if (!live) return;
      if (!read.ok && shownView.current?.ok === true) {
        toast(read.problem.message, "error");
      } else {
        shownView.current = read;
        setView(read);
      }
      if (activity.ok || shownTimeline.current?.ok !== true) {
        shownTimeline.current = activity;
        setTimeline(activity);
      }
      setReadAt(Date.now());
      setRefreshing(false);
    });
    return () => {
      live = false;
    };
  }, [fleet, node, session, asked]);

  const refresh = useCallback(() => {
    setRefreshing(true);
    setAsked((count) => count + 1);
  }, []);
  const report = useCallback(
    (run: RunId) =>
      attempt(() => fleet.bridge.call(node, "pr.check", { session, run })),
    [fleet, node, session],
  );
  return { view, timeline, refreshing, readAt, refresh, report };
}
