// The slot after a sidebar row's name (Paseo's
// `components/sidebar/workspace-trailing/index.tsx`). Diff when a stat
// is served, last activity in Paseo's compact age, or nothing.

import { useEffect, useState } from "react";

import { compactDelay, compactTimeAgo } from "../../data/time";
import { formatDiffCount } from "../../data/workspace/diff-stat";
import type { DiffStatState } from "../../data/workspace/diff-stat-hub";
import { workspaceOf } from "../../data/workspace-labels/groups";
import type { LabelsState } from "../../data/workspace-labels/hub";
import type { NodeKey, SidebarTrailing } from "../../wire/bridge";
import type { DiffStat, SessionInfo } from "../../wire/proto";

/** The diff stat of the workspace `session` belongs to, when one is served. */
export function statForSession(
  labels: LabelsState,
  stats: DiffStatState,
  node: NodeKey,
  session: SessionInfo | undefined,
): DiffStat | null | undefined {
  if (session === undefined) {
    return undefined;
  }
  const snapshot = labels.registries[node];
  if (snapshot === undefined) {
    return undefined;
  }
  const workspace = workspaceOf(snapshot, session);
  if (workspace === null) {
    return undefined;
  }
  return stats.byNode[node]?.[workspace];
}

export function sessionOn(
  sessions: readonly SessionInfo[] | undefined,
  id: string,
): SessionInfo | undefined {
  return sessions?.find((session) => session.id === id);
}

/** What the slot draws. Null when the preference or the row has nothing. */
export function SidebarTrailingSlot({
  trailing,
  activity,
  stat,
}: {
  trailing: SidebarTrailing;
  /** The activity this row shows. Null draws no timestamp. */
  activity: string | null;
  stat: DiffStat | null | undefined;
}) {
  if (trailing === "diff") {
    return stat == null ? null : <DiffCounts stat={stat} />;
  }
  if (trailing === "timestamp" && activity !== null) {
    return <ActivityStamp activity={activity} />;
  }
  return null;
}

function DiffCounts({ stat }: { stat: DiffStat }) {
  return (
    <span className="side-trailing workspace-card-diff">
      <span className="workspace-card-add">
        {`+${formatDiffCount(stat.additions)}`}
      </span>
      <span className="workspace-card-del">
        {`\u2212${formatDiffCount(stat.deletions)}`}
      </span>
    </span>
  );
}

function ActivityStamp({ activity }: { activity: string }) {
  const label = useCompactStamp(activity);
  return <span className="side-trailing">{label}</span>;
}

function useCompactStamp(activity: string): string {
  const [now, setNow] = useState(() => Date.now());
  const age = compactTimeAgo(new Date(activity), new Date(now));
  useEffect(() => {
    const wait = compactDelay(age.resolution);
    if (wait === null) {
      return undefined;
    }
    const handle = window.setTimeout(() => {
      setNow(Date.now());
    }, wait);
    return () => {
      window.clearTimeout(handle);
    };
  }, [activity, age.resolution, now]);
  return age.label;
}
