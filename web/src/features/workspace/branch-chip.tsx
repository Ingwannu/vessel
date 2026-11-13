// Where a session's code is, in one small chip: the branch of its
// worktree (GOAT's branch selector in its toolbar, agent-orchestrator's
// branch on a card), or the project it runs in. On a session's bar it
// opens the code panel's Changes; on a card and a row it only says.

import "./branch-chip.css";

import { useFleet } from "../../context";
import {
  projectAt,
  projectIn,
  useProjects,
} from "../../data/workspace/project-store";
import { Icon } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";

export function BranchChip({
  branch,
  project,
  onOpen,
}: {
  /** The worktree's branch, when the session has one. */
  branch: string | null;
  /** The project's name, said when there is no branch. */
  project?: string | null;
  /** Opens the session's changes (the bar's chip); none on a card. */
  onOpen?: () => void;
}) {
  const text = branch ?? project ?? null;
  if (text === null) {
    return null;
  }
  const body = (
    <>
      <Icon name={branch === null ? "folder" : "git-branch"} size={12} />
      <span className="branch-chip-text">{text}</span>
    </>
  );
  return onOpen === undefined ? (
    <span className="branch-chip">{body}</span>
  ) : (
    <button
      type="button"
      className="branch-chip action"
      aria-label={`${branch === null ? "Project" : "Branch"} ${text}: show changes`}
      onClick={onOpen}
    >
      {body}
    </button>
  );
}

/** A session's chip, its project's name looked up in the machine's list
 * when it runs in a project's folder. */
export function SessionPlaceChip({
  node,
  session,
  onOpen,
}: {
  node: NodeKey;
  session: SessionInfo;
  onOpen?: () => void;
}) {
  const fleet = useFleet();
  const workspace = session.workspace ?? null;
  const projects = useProjects(fleet, node);
  // A session started in a typed folder inside a project's is that
  // project's too (the panel reads it the same way, panel.tsx).
  const project =
    workspace === null
      ? projectAt(projects, session.spec.cwd)
      : projectIn(projects, workspace.project);
  return (
    <BranchChip
      branch={workspace?.branch ?? null}
      project={project?.name ?? null}
      {...(onOpen === undefined ? {} : { onOpen })}
    />
  );
}
