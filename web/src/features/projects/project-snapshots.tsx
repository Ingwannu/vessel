// A project's snapshots in Settings › Projects: what a worktree removed
// with a snapshot kept (GOAT's), each dropped only after a second press,
// since a snapshot is the only copy of that work; a drop the node refuses
// is said in words above the list.

import { useState } from "react";

import { useFleet } from "../../context";
import { when } from "../../data/format";
import { dropSnapshot } from "../../data/workspace/worktrees";
import { Row, RowList } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { ProjectId, Snapshot } from "../../wire/proto";
import { SettingsSection } from "../settings/settings-page";
import { ConfirmDrop } from "../workspace/confirm-drop";

export function ProjectSnapshots({
  node,
  project,
  snapshots,
  onDropped,
  onFailed,
}: {
  node: NodeKey;
  project: ProjectId;
  snapshots: readonly Snapshot[];
  onDropped: () => void;
  onFailed: (message: string) => void;
}) {
  const fleet = useFleet();
  const [dropping, setDropping] = useState<Snapshot["worktree"] | null>(null);
  if (snapshots.length === 0) {
    return null;
  }
  return (
    <SettingsSection title="Snapshots">
      <RowList label="Snapshots">
        {snapshots.map((snapshot) => {
          const name = snapshot.branch ?? snapshot.commit.slice(0, 7);
          return (
            <Row
              key={snapshot.worktree}
              mark={{ icon: "archive" }}
              title={name}
              detail={{
                facts: [snapshot.commit.slice(0, 7), when(snapshot.created_at)],
              }}
              control={
                <ConfirmDrop
                  what={`the snapshot of ${name}`}
                  busy={dropping === snapshot.worktree}
                  disabled={dropping !== null}
                  onDrop={() => {
                    setDropping(snapshot.worktree);
                    void dropSnapshot(
                      fleet,
                      node,
                      project,
                      snapshot.worktree,
                    ).then((dropped) => {
                      setDropping(null);
                      if (!dropped.ok) {
                        onFailed(
                          `The snapshot of ${name} was not dropped: ${dropped.problem.message}`,
                        );
                      }
                      onDropped();
                    });
                  }}
                />
              }
            />
          );
        })}
      </RowList>
    </SettingsSection>
  );
}
