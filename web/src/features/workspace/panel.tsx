// A session's code panel, Changes and Files (the inspector's tabs beside
// Summary): a screen of its own, loaded the first time one of the two is
// opened (ADR 0017's amendment: the chunk is pinned by the index's import
// map like every other). It reads the session's worktree, its project's
// folder, or the project whose folder holds the session's own; a session
// in no project's folder is offered to add its folder as one.

import { useState } from "react";

import { useFleet } from "../../context";
import { projectIn, tookProject } from "../../data/workspace/project-store";
import { addProject } from "../../data/workspace/projects";
import { rootKey } from "../../data/workspace/root";
import { Button, EmptyState, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionInfo } from "../../wire/proto";
import { ChangesTab } from "./changes-tab";
import { useSessionRoot } from "./use-session-root";
import { FilesTab, type Opened } from "./files-tab";

function NoProject({ node, session }: { node: NodeKey; session: SessionInfo }) {
  const fleet = useFleet();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const cwd = session.spec.cwd ?? null;
  return (
    <EmptyState
      title="Not in a project"
      icon="folder"
      {...(cwd === null
        ? {}
        : {
            action: (
              <Button
                icon="folder-plus"
                busy={busy}
                onClick={() => {
                  setBusy(true);
                  void addProject(fleet, node, cwd, null).then((added) => {
                    setBusy(false);
                    if (added.ok) {
                      tookProject(fleet, node, added.value);
                    } else {
                      setError(added.problem.message);
                    }
                  });
                }}
              >
                Add the folder as a project
              </Button>
            ),
          })}
    >
      {error ??
        (cwd === null
          ? "This session runs in its machine's home folder, which is not a project."
          : `This session runs in ${cwd}, which no project of this machine holds. Add it as a project to see its changes and files here.`)}
    </EmptyState>
  );
}

export function WorkspacePanel({
  node,
  session,
  tab,
  opened,
  onOpened,
}: {
  node: NodeKey;
  session: SessionInfo;
  tab: "changes" | "files";
  opened: Opened | null;
  /** A file opened (from the tree, a search, or a diff's header), or
   * `null` back to the tree; opening one shows the Files tab. */
  onOpened: (opened: Opened | null) => void;
}) {
  // One object while it names the same place: the tabs' reads (the
  // comparison's poll, a file's pages, a search's settle) are effects of
  // the root (use-session-root.ts).
  const { root, projects } = useSessionRoot(node, session);
  if (root === null) {
    return projects.kind === "loading" ? (
      <div className="changes-pad">
        <Skeleton />
      </div>
    ) : (
      <NoProject node={node} session={session} />
    );
  }
  const owner =
    root.kind === "project"
      ? root.project
      : (session.workspace?.project ?? null);
  const project = owner === null ? null : projectIn(projects, owner);
  // The root's folder: the project's; a worktree's is where the node
  // started the session (its cwd).
  const folder =
    root.kind === "project"
      ? (project?.root ?? null)
      : (session.spec.cwd ?? null);
  const open = (path: RelPath) => {
    onOpened({ path, goTo: null });
  };
  return tab === "changes" ? (
    <ChangesTab
      key={rootKey(root)}
      node={node}
      session={session.id}
      root={root}
      onOpen={open}
      defaultBranch={project?.default_branch ?? null}
    />
  ) : (
    <FilesTab
      key={rootKey(root)}
      node={node}
      root={root}
      folder={folder}
      session={session.id}
      opened={opened}
      onOpened={onOpened}
    />
  );
}
