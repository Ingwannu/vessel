// Which comparison the Changes tab shows (agent-orchestrator's changes
// picker): a quiet trigger with the comparison's name and its counts, a
// menu of the comparisons (everything since the base, uncommitted, staged,
// unstaged) and, under "Commits", the latest commits one by one.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import {
  type ScopeKind,
  scopeDetail,
  scopeLabel,
} from "../../data/workspace/changes";
import { readLog } from "../../data/workspace/git";
import {
  Button,
  Menu,
  MenuItem,
  MenuRadioGroup,
  MenuSeparator,
  MenuSubmenu,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { CommitInfo, DiffScope, WorkspaceRoot } from "../../wire/proto";

/** How many commits the submenu lists (agent-orchestrator shows a
 * window of them; the node pages 200). */
const COMMITS = 20;

const KINDS: readonly ScopeKind[] = [
  "base",
  "uncommitted",
  "staged",
  "unstaged",
];

export function ScopeMenu({
  node,
  root,
  scope,
  onScope,
  added,
  removed,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  scope: DiffScope;
  onScope: (scope: DiffScope) => void;
  added: number;
  removed: number;
}) {
  const fleet = useFleet();
  const [commits, setCommits] = useState<readonly CommitInfo[]>([]);
  useEffect(() => {
    let current = true;
    void readLog(fleet, node, root, 0).then((outcome) => {
      if (current && outcome.ok) {
        setCommits(outcome.value.commits.slice(0, COMMITS));
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, node, root]);
  const value: ScopeKind | "commit" =
    scope.kind === "commit" ? "commit" : scope.kind;
  return (
    <Menu
      width="wide"
      trigger={
        <Button
          size="xs"
          variant="ghost"
          end={
            <span className="scope-counts">
              <span className="d-add">+{String(added)}</span>
              <span className="d-del">-{String(removed)}</span>
            </span>
          }
        >
          {scopeLabel(scope)}
        </Button>
      }
    >
      {/* A commit chosen below leaves none of these marked. */}
      <MenuRadioGroup<ScopeKind | "commit">
        value={value}
        onChange={(kind) => {
          if (kind !== "commit") {
            onScope({ kind });
          }
        }}
        options={KINDS.map((kind) => ({
          value: kind,
          label: scopeLabel({ kind }),
          detail: scopeDetail(kind),
        }))}
      />
      <MenuSeparator />
      <MenuSubmenu
        label="Commits"
        icon="git-commit"
        {...(scope.kind === "commit" ? { value: scope.id.slice(0, 7) } : {})}
      >
        {commits.length === 0 ? (
          <MenuItem disabled onSelect={() => undefined}>
            No commits yet
          </MenuItem>
        ) : (
          commits.map((commit) => (
            <MenuItem
              key={commit.id}
              description={`${commit.id.slice(0, 7)} · ${commit.author}`}
              onSelect={() => {
                onScope({ kind: "commit", id: commit.id });
              }}
            >
              {commit.subject}
            </MenuItem>
          ))
        )}
      </MenuSubmenu>
    </Menu>
  );
}
