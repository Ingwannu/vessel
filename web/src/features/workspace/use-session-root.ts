// Which project or worktree a session's code is read in: its own
// worktree, else its project, else (a session started in a typed folder)
// the project whose folder holds it. The code panel and the composer's
// `@` both read it here, so the two never disagree about a session.
//
// [Decision Log]
// - Purpose: track B-page's panel and track A-page's `@` each worked out
//   a session's root; the panel also took a typed folder inside a
//   project as that project, `@` did not, and `@` made a new root object
//   on every drawing, which its search's effect read as a new root and
//   asked `file.find` again after every answer (the join's review).
// - Choice: one hook, the root one object while it names the same place
//   (the session's page draws again on every record the agent streams).
// - Cost: the composer reads the machine's projects (one shared store,
//   `project-store.ts`, read once per machine).

import { useMemo } from "react";

import { useFleet } from "../../context";
import {
  projectAt,
  type ProjectsView,
  useProjects,
} from "../../data/workspace/project-store";
import { sessionRoot } from "../../data/workspace/root";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo, WorkspaceRoot } from "../../wire/proto";

/** The root `session`'s code is read in (`null`: in no project), and the
 * machine's projects it was found among. */
export function useSessionRoot(
  node: NodeKey,
  session: SessionInfo,
): { root: WorkspaceRoot | null; projects: ProjectsView } {
  const fleet = useFleet();
  const projects = useProjects(fleet, node);
  const placed = sessionRoot(session);
  const found = placed === null ? projectAt(projects, session.spec.cwd) : null;
  const projectId =
    placed?.kind === "project" ? placed.project : (found?.id ?? null);
  const worktreeId = placed?.kind === "worktree" ? placed.worktree : null;
  const root = useMemo<WorkspaceRoot | null>(
    () =>
      worktreeId !== null
        ? { kind: "worktree", worktree: worktreeId }
        : projectId === null
          ? null
          : { kind: "project", project: projectId },
    [projectId, worktreeId],
  );
  return { root, projects };
}
