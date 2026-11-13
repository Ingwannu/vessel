// A machine's projects and their commands (`project.*`, protocol 10):
// typed calls in the shape of `data/calls.ts`, a typed result or a typed
// failure, nothing thrown.
//
// [Decision Log]
// - Purpose: the sidebar's projects, Settings › Projects, the
//   new-session page and the approval dialog all list, add, clone and
//   approve; each would otherwise write its own `bridge.call`.
// - Constraint: components may not import the bridge (`npm run lint`),
//   and a refusal must stay typed so a screen can say "this folder is
//   not empty" without reading text.
// - Choice: one function per call, named for what it does; the node's
//   own words travel in the outcome's problem (`data/outcome.ts`).
// - Cost: one indirection per call.

import type { NodeKey } from "../../wire/bridge";
import type {
  CommandsDigest,
  ProjectCommands,
  ProjectId,
  ProjectInfo,
  ProjectList,
  WorktreeBase,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** Every project of `node`, oldest first, and which git it has. */
export function listProjects(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<ProjectList>> {
  return attempt(() => fleet.bridge.call(node, "project.list", {}));
}

/** One project. With `changedFrom` (a revision the page has), the node
 * answers when the project's revision differs from it, or after at most
 * 30 s as it is: how a clone's progress is followed. */
export function readProject(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
  changedFrom: number | null = null,
): Promise<Outcome<ProjectInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.get", {
      project,
      ...(changedFrom === null ? {} : { changed_from: changedFrom }),
    }),
  );
}

/** Add the folder `path` (absolute, on `node`) as a project; inside a
 * repository, its top level is the root. A root already known answers
 * that project. */
export function addProject(
  fleet: Fleet,
  node: NodeKey,
  path: string,
  name: string | null,
): Promise<Outcome<ProjectInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.add", {
      path,
      ...(name === null ? {} : { name }),
    }),
  );
}

/** Clone `url` into `parent` (absolute, on `node`), in `folder` or the
 * folder the URL names. Answers at once with the project `cloning`; the
 * clone continues on the machine when the page goes away. */
export function cloneProject(
  fleet: Fleet,
  node: NodeKey,
  clone: { url: string; parent: string; folder: string | null },
): Promise<Outcome<ProjectInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.clone", {
      url: clone.url,
      parent: clone.parent,
      ...(clone.folder === null ? {} : { folder: clone.folder }),
    }),
  );
}

/** Start a new project: create `path` if needed (it must be empty) and
 * run `git init` there. */
export function initProject(
  fleet: Fleet,
  node: NodeKey,
  path: string,
  name: string | null,
): Promise<Outcome<ProjectInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.init", {
      path,
      ...(name === null ? {} : { name }),
    }),
  );
}

/** Forget a project; its files stay. Refused while it has worktrees or
 * live sessions; a clone still running is stopped. */
export function removeProject(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
): Promise<Outcome<null>> {
  return attempt(() => fleet.bridge.call(node, "project.remove", { project }));
}

/** The commands the project's `vessel.toml` names at `base` (read from
 * the commit, never a checkout) or in its folder as it is, and whether
 * that exact set is approved. */
export function projectCommands(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
  base: WorktreeBase | null = null,
): Promise<Outcome<ProjectCommands>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.commands", {
      project,
      ...(base === null ? {} : { base }),
    }),
  );
}

/** Approve (or, with `approved: false`, withdraw) the set of commands
 * whose digest is `digest`: the one the person was shown. Answers the
 * folder's commands. */
export function approveCommands(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
  digest: CommandsDigest,
  approved: boolean,
): Promise<Outcome<ProjectCommands>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.approve", { project, digest, approved }),
  );
}
