// The project a new session starts in when nothing asks for another
// (the new chat, 2026-11-13): GOAT opens its first draft "in the most
// recently updated project" (`selectMostRecentOpenProject`), and
// agent-orchestrator's ⌘N the open session's project, else the last one
// used. The page had started every new session at "No project", in the
// machine's home folder.
//
// [Decision Log]
// - Purpose: Send right away, in the project the person works in.
// - Constraint: the bridge keeps a machine's last agent and choices, not
//   its last place (its preferences take no place; adding one changes
//   the bridge's keeping, outside the page).
// - Alternatives: remember the last project in this tab (lost on reload,
//   and unlike every other device of the person); keep "No project".
// - Choice: the project of the machine's most recently active session
//   that is still one of its projects: what the node already says, the
//   same on every device, and GOAT's rule. The project of the session in
//   view, when New is pressed from it, comes first (wanted-place.ts).
// - Cost: a project last used with no session in it (a session deleted)
//   is not found; nothing remembered then, "No project" as before.

import type { ProjectId, SessionInfo } from "../../wire/proto";
import { activeAt } from "../sidebar";
import { projectIn, type ProjectsView } from "./project-store";

/** The project of the most recently active of `sessions` that is still
 * one of `projects`; `null` while the projects are not read, or when no
 * session ran in one. */
export function lastProject(
  sessions: readonly SessionInfo[] | undefined,
  projects: ProjectsView,
): ProjectId | null {
  if (projects.kind !== "ready" || sessions === undefined) {
    return null;
  }
  let newest: { at: string; project: ProjectId } | null = null;
  for (const session of sessions) {
    const project = session.workspace?.project;
    if (project === undefined || projectIn(projects, project) === null) {
      continue;
    }
    const at = activeAt(session);
    if (newest === null || at > newest.at) {
      newest = { at, project };
    }
  }
  return newest?.project ?? null;
}
