// The repository's stash (Paseo's stash list; GOAT has none): each entry
// with when it was made, and Apply, Pop and Drop. An entry is named by its
// index and its commit, so an entry that moved since it was listed is
// refused (`stash_moved`) rather than the wrong one changed.

import { useCallback, useEffect, useState } from "react";

import { useFleet } from "../../context";
import { when } from "../../data/format";
import { readStashes } from "../../data/workspace/git";
import { Button, Dialog, Spinner } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { StashEntry, WorkspaceRoot } from "../../wire/proto";
import { ConfirmDrop } from "./confirm-drop";
import { GitNotice } from "./git-notice";
import type { Git } from "./use-git";

export function StashDialog({
  node,
  root,
  git,
  onClose,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  git: Git;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [entries, setEntries] = useState<readonly StashEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => {
    void readStashes(fleet, node, root).then((outcome) => {
      if (outcome.ok) {
        setEntries(outcome.value.entries);
        setError(null);
      } else {
        setError(outcome.problem.message);
      }
    });
  }, [fleet, node, root]);
  useEffect(load, [load]);
  const act = (entry: StashEntry, kind: "apply" | "pop" | "drop") => {
    void git
      .stash({ kind, index: entry.index, commit: entry.commit })
      .then(load);
  };
  return (
    <Dialog title="Stashes" onClose={onClose}>
      {error !== null ? (
        <p className="branch-empty">{error}</p>
      ) : entries === null ? (
        <p className="branch-empty">
          <Spinner size={12} /> Loading the stash…
        </p>
      ) : entries.length === 0 ? (
        <p className="branch-empty">Nothing is stashed in this repository.</p>
      ) : (
        <ul className="stash-list" aria-label="Stash entries">
          {entries.map((entry) => (
            <li
              key={`${String(entry.index)}:${entry.commit}`}
              className="stash-row"
            >
              <span className="stash-text">
                <span className="stash-message">{entry.message}</span>
                <span className="stash-meta">
                  stash@{`{${String(entry.index)}}`} · {when(entry.time)}
                </span>
              </span>
              <Button
                size="xs"
                disabled={git.busy !== null}
                onClick={() => {
                  act(entry, "apply");
                }}
              >
                Apply
              </Button>
              <Button
                size="xs"
                disabled={git.busy !== null}
                onClick={() => {
                  act(entry, "pop");
                }}
              >
                Pop
              </Button>
              <ConfirmDrop
                what={`stash@{${String(entry.index)}}`}
                busy={git.busy === "stash"}
                disabled={git.busy !== null}
                onDrop={() => {
                  act(entry, "drop");
                }}
              />
            </li>
          ))}
        </ul>
      )}
      {git.notice === null ? null : <GitNotice notice={git.notice} />}
    </Dialog>
  );
}
