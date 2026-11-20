import { lazy, Suspense, useCallback } from "react";

import type { ProjectsView } from "../../data/workspace/project-store";
import { tabKey } from "../../data/workspace/tab-layout";
import { Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionInfo, WorkspaceRoot } from "../../wire/proto";
import type { CodeTarget } from "../workspace/panel";

const WorkspacePanel = lazy(async () => {
  const module = await import("../workspace/panel");
  return { default: module.WorkspacePanel };
});

export function CodePane({
  node,
  session,
  root,
  projects,
  tab,
  active,
  goTo,
  open,
  back,
  markDirty,
}: {
  node: NodeKey;
  session: SessionInfo;
  root: WorkspaceRoot | null;
  projects: ProjectsView;
  tab: CodeTarget;
  active: boolean;
  goTo: { line: number; seq: number } | null;
  open: (path: RelPath, line: number | null) => void;
  back: () => void;
  markDirty: (key: string, dirty: boolean) => void;
}) {
  const key = tabKey(tab);
  const dirty = useCallback(
    (value: boolean) => markDirty(key, value),
    [key, markDirty],
  );
  return (
    <Suspense
      fallback={
        <div className="changes-pad">
          <Skeleton />
        </div>
      }
    >
      <WorkspacePanel
        node={node}
        session={session}
        root={root}
        projects={projects}
        tab={tab}
        active={active}
        goTo={goTo}
        onOpen={open}
        onBack={back}
        onDirtyChange={dirty}
      />
    </Suspense>
  );
}
