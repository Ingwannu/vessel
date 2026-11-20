// Loads code viewers only after a workspace tab asks for them. Every
// viewer uses the same root as the composer and keeps its existing RPCs.

import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { projectIn, tookProject } from "../../data/workspace/project-store";
import { addProject } from "../../data/workspace/projects";
import { hasFilePanels } from "../../data/workspace/files";
import { Button, EmptyState, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionInfo, WorkspaceRoot } from "../../wire/proto";
import { ChangesTab } from "./changes-tab";
import type { ProjectsView } from "../../data/workspace/project-store";
import { FileView } from "./file-view";
import { FilesTab } from "./files-tab";

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

export type CodeTarget =
  { kind: "changes" | "diff" | "files" } | { kind: "file"; path: RelPath };

export function WorkspacePanel({
  node,
  session,
  root,
  projects,
  tab,
  goTo,
  active,
  onOpen,
  onBack,
  onDirtyChange,
}: {
  node: NodeKey;
  session: SessionInfo;
  root: WorkspaceRoot | null;
  projects: ProjectsView;
  tab: CodeTarget;
  goTo: { line: number; seq: number } | null;
  active: boolean;
  onOpen: (path: RelPath, line: number | null) => void;
  onBack: () => void;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
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
  const folder =
    root.kind === "project"
      ? (project?.root ?? null)
      : (session.spec.cwd ?? null);
  switch (tab.kind) {
    case "changes":
    case "diff":
      return (
        <ChangesTab
          node={node}
          session={session.id}
          root={root}
          view={tab.kind}
          onOpen={(path) => onOpen(path, null)}
          defaultBranch={project?.default_branch ?? null}
        />
      );
    case "files":
      return (
        <FilesTab
          node={node}
          root={root}
          folder={folder}
          session={session.id}
          onOpen={onOpen}
        />
      );
    case "file":
      return (
        <FileView
          node={node}
          root={root}
          folder={folder}
          session={session.id}
          path={tab.path}
          goTo={goTo}
          filePanels={hasFilePanels(protocol)}
          active={active}
          onBack={onBack}
          onOpenFile={(path) => onOpen(path, null)}
          onDirtyChange={onDirtyChange}
        />
      );
  }
}
