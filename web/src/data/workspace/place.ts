// Where a new session runs (the new-session page's tray): a typed folder,
// or a project's folder as it is, a new worktree on a new branch from a
// base, or a worktree on an existing branch (agent-orchestrator's spawn
// screen and GOAT's environment selector, Paseo's "where").
//
// [Decision Log]
// - Purpose: wave 1 gave the node projects and a worktree per session;
//   the page still started a session from a typed path.
// - Constraint: a session with a `place` has no `cwd` (the node fills
//   it); a worktree must exist, and its setup must have ended, before
//   `session.create` names it (the node answers `conflict` while setup
//   runs). A machine older than protocol 10 knows no projects.
// - Alternatives: a place as a free-form path with a project guessed from
//   it (what the folder field did): a guess cannot pick a branch.
// - Choice: a closed set of places here, the spec's part made from it,
//   and whether it needs a worktree first; the steps are `start.ts`'s.
// - Cost: the typed folder stays for machines without projects and for a
//   folder no project holds, so there are two ways to name a folder.

import type {
  BranchName,
  ProjectId,
  ProjectInfo,
  SessionSpec,
  SetupChoice,
  WorktreeBranch,
} from "../../wire/proto";
import { branchNameProblem } from "./branch-name";

/** How a session uses a project. */
export type PlaceMode = "folder" | "worktree" | "branch";

/** A project chosen in the tray. */
export interface ProjectPlace {
  kind: "project";
  project: ProjectId;
  mode: PlaceMode;
  /** A new worktree's base (`null`: the project's default branch). */
  base: BranchName | null;
  /** A new worktree's branch name (`""`: the machine names it), or the
   * existing branch to run on. */
  branch: string;
  setup: SetupChoice;
}

/** What the tray holds. */
export type Place =
  /** No project: the folder typed (`""`: the machine's home folder). */
  { kind: "path"; folder: string } | ProjectPlace;

/** The tray before anything is chosen. */
export const NO_PLACE: Place = { kind: "path", folder: "" };

/** A project chosen, in its folder as it is, as agent-orchestrator's
 * "Local" place: the least surprising start. */
export function projectPlace(project: ProjectId): ProjectPlace {
  return {
    kind: "project",
    project,
    mode: "folder",
    base: null,
    branch: "",
    setup: "run",
  };
}

/** The spec's part for `place` when it runs without a new worktree. */
export function placeSpec(
  place: Place,
): Pick<SessionSpec, "cwd" | "place"> | null {
  if (place.kind === "path") {
    const folder = place.folder.trim();
    return folder === "" ? {} : { cwd: folder };
  }
  return place.mode === "folder"
    ? { place: { kind: "project", project: place.project } }
    : null;
}

/** The branch a worktree for `place` is made on, or `null` when the
 * place needs no worktree. */
export function worktreeBranch(place: Place): WorktreeBranch | null {
  if (place.kind === "path") {
    return null;
  }
  const name = place.branch.trim();
  switch (place.mode) {
    case "folder":
      return null;
    case "worktree":
      return {
        kind: "new",
        ...(name === "" ? {} : { name }),
        ...(place.base === null
          ? {}
          : { base: { kind: "ref", name: place.base } }),
      };
    case "branch":
      return { kind: "existing", name };
  }
}

/** Why Send cannot start in `place` yet (`null`: it can). `project` is
 * the chosen project as last listed, `null` when it is not. */
export function placeProblem(
  place: Place,
  project: ProjectInfo | null,
): string | null {
  if (place.kind === "path") {
    return null;
  }
  if (project === null) {
    return "That project is not on this machine.";
  }
  switch (project.state.state) {
    case "ready":
      break;
    case "cloning":
      return "The project is still being cloned.";
    case "clone_failed":
      return "The project's clone failed; clone it again from Settings › Projects.";
    case "missing":
      return "The project's folder is gone.";
    case "unknown":
      return "The project is in a state this page does not know.";
  }
  if (place.mode === "folder") {
    return null;
  }
  if (!project.git) {
    return "This project is not a git repository: run in its folder.";
  }
  const name = place.branch.trim();
  if (place.mode === "branch" && name === "") {
    return "Choose the branch to run on.";
  }
  return name === "" ? null : branchNameProblem(name);
}
