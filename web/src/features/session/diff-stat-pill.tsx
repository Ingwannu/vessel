// The composer's diff pill (Paseo's `composer/diff-stat-pill.tsx` and
// `composer/workspace-diff-stat.ts`). Shown only when a side is greater
// than 0. Named "Open Changes tab". The press opens the existing Changes
// tab; there is no second diff pane.

import { useContext } from "react";

import { formatDiffCount } from "../../data/workspace/diff-stat";
import { workspaceOf } from "../../data/workspace-labels/groups";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";
import { useDiffStats } from "../shell/use-diff-stats";
import { OpenChanges } from "../workspace/workspace-chat";
import { useLabels } from "../workspace-labels/use-labels";

export function DiffStatPill({
  node,
  session,
}: {
  node: NodeKey;
  session: SessionInfo;
}) {
  const { state } = useLabels();
  const stats = useDiffStats();
  const open = useContext(OpenChanges);
  const snapshot = state.registries[node];
  const workspace =
    snapshot === undefined ? null : workspaceOf(snapshot, session);
  const stat = workspace === null ? undefined : stats.byNode[node]?.[workspace];
  if (
    stat == null ||
    open === null ||
    (stat.additions <= 0 && stat.deletions <= 0)
  ) {
    return null;
  }
  return (
    <button
      type="button"
      className="composer-diff-pill"
      aria-label="Open Changes tab"
      onClick={open}
    >
      <span className="workspace-card-add">
        {`+${formatDiffCount(stat.additions)}`}
      </span>
      <span className="workspace-card-del">
        {`\u2212${formatDiffCount(stat.deletions)}`}
      </span>
    </button>
  );
}
