// Each machine's projects as the page last read them, shared by the
// sidebar's group, Settings › Projects, the new-session tray and a
// session's panel; a clone is followed until it ends.
//
// [Decision Log]
// - Purpose: four screens show the same list; each asking `project.list`
//   on its own would read it four times and show four states.
// - Constraint: the node pushes nothing when a project changes; a clone's
//   progress is followed with `project.get { changed_from }` (a wait of at
//   most 30 s on the node, answered when the revision differs). A machine
//   older than protocol 10 has no projects at all.
// - Alternatives: the fleet's own store (`data/fleet.ts`, polled every
//   two seconds for sessions): projects change rarely, and that file is
//   shared by every track.
// - Choice: a store of its own, read when a screen first needs a
//   machine's list and again after any change made from the page; one
//   follower per cloning project, which ends when the clone does.
// - Cost: a project added from the CLI shows when a screen reads again
//   (opening Settings › Projects, the new-session page, or a refresh).

import { useEffect } from "react";

import type { NodeKey } from "../../wire/bridge";
import type { ProjectId, ProjectInfo, ProjectList } from "../../wire/proto";
import type { Fleet } from "../fleet";
import type { Problem } from "../outcome";
import { Store, useStore } from "../store";
import { quiet } from "./pace";
import { listProjects, readProject } from "./projects";

/** What the page knows of a machine's projects. */
export type ProjectsView =
  | { kind: "loading" }
  | { kind: "ready"; list: ProjectList }
  | { kind: "failed"; problem: Problem }
  /** The machine speaks a protocol older than projects. */
  | { kind: "older" };

const LOADING: ProjectsView = { kind: "loading" };

/** Every machine's view, by machine. */
export const projectStore = new Store<
  Readonly<Partial<Record<NodeKey, ProjectsView>>>
>({});

/** Projects being followed while they clone, by machine and id. */
const following = new Set<string>();

function put(node: NodeKey, view: ProjectsView): void {
  projectStore.update((all) => ({ ...all, [node]: view }));
}

/** `project` as it is now, in the machine's list. */
function replace(node: NodeKey, project: ProjectInfo): void {
  const view = projectStore.get()[node];
  if (view?.kind !== "ready") {
    return;
  }
  put(node, {
    kind: "ready",
    list: {
      ...view.list,
      projects: view.list.projects.map((known) =>
        known.id === project.id ? project : known,
      ),
    },
  });
}

/** Follow a cloning project until its clone ends (or the page cannot
 * ask), keeping the list's entry current. */
async function follow(fleet: Fleet, node: NodeKey, project: ProjectInfo) {
  const key = `${node}\u0000${project.id}`;
  if (following.has(key)) {
    return;
  }
  following.add(key);
  let revision = project.revision;
  try {
    for (;;) {
      const asked = performance.now();
      const read = await readProject(fleet, node, project.id, revision);
      if (!read.ok) {
        return;
      }
      replace(node, read.value);
      if (read.value.state.state !== "cloning") {
        return;
      }
      if (read.value.revision === revision) {
        // Answered without waiting, with nothing new (pace.ts).
        await quiet(asked, null);
      }
      revision = read.value.revision;
    }
  } finally {
    following.delete(key);
  }
}

/** Read `node`'s projects (again), and follow the ones cloning. */
export async function loadProjects(fleet: Fleet, node: NodeKey): Promise<void> {
  const listed = await listProjects(fleet, node);
  if (!listed.ok) {
    put(
      node,
      listed.problem.from === "call" &&
        listed.problem.failure.kind === "not_in_protocol"
        ? { kind: "older" }
        : { kind: "failed", problem: listed.problem },
    );
    return;
  }
  put(node, { kind: "ready", list: listed.value });
  for (const project of listed.value.projects) {
    if (project.state.state === "cloning") {
      void follow(fleet, node, project);
    }
  }
}

/** Take a project the node just answered with (added, cloned, started)
 * into the list at once, and follow it if it clones. */
export function tookProject(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectInfo,
): void {
  const view = projectStore.get()[node];
  if (view?.kind === "ready") {
    const known = view.list.projects.some((each) => each.id === project.id);
    put(node, {
      kind: "ready",
      list: {
        ...view.list,
        projects: known
          ? view.list.projects.map((each) =>
              each.id === project.id ? project : each,
            )
          : [...view.list.projects, project],
      },
    });
  }
  if (project.state.state === "cloning") {
    void follow(fleet, node, project);
  }
}

/** `node`'s projects, read the first time a screen asks. */
export function useProjects(fleet: Fleet, node: NodeKey | null): ProjectsView {
  const view = useStore(projectStore, (all) =>
    node === null ? LOADING : (all[node] ?? LOADING),
  );
  useEffect(() => {
    // Multiple rows can ask in the same effect flush. Check the live
    // store so only the first starts this machine's read.
    if (node !== null && projectStore.get()[node] === undefined) {
      put(node, LOADING);
      void loadProjects(fleet, node);
    }
  }, [fleet, node]);
  return view;
}

/** A project of `view` by id. */
export function projectIn(
  view: ProjectsView,
  id: ProjectId,
): ProjectInfo | null {
  return view.kind === "ready"
    ? (view.list.projects.find((project) => project.id === id) ?? null)
    : null;
}

/** The project whose folder holds `cwd` (the deepest, when one project
 * is inside another), for a session started in a typed folder. Folders
 * are compared as text: a link to a project's folder is not followed. */
export function projectAt(
  view: ProjectsView,
  cwd: string | null | undefined,
): ProjectInfo | null {
  return view.kind === "ready" ? projectAmong(view.list.projects, cwd) : null;
}

/** The project of `projects` whose folder holds `cwd`, the deepest. */
export function projectAmong(
  projects: readonly ProjectInfo[],
  cwd: string | null | undefined,
): ProjectInfo | null {
  if (cwd === null || cwd === undefined) {
    return null;
  }
  const folder = cwd.replace(/[\\/]+$/u, "");
  let best: ProjectInfo | null = null;
  for (const project of projects) {
    const root = project.root.replace(/[\\/]+$/u, "");
    const inside =
      folder === root ||
      folder.startsWith(`${root}/`) ||
      folder.startsWith(`${root}\\`);
    if (inside && (best === null || root.length > best.root.length)) {
      best = project;
    }
  }
  return best;
}
