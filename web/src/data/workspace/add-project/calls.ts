// The Add project sheet's calls that `data/workspace/projects.ts` does not
// have: the home's folders for the field (`folder.suggest`, ADR 0112) and
// a new empty directory made a project (`project.create_directory`,
// ADR 0113). The protocols that gate them are the generated
// `HOST_FOLDERS_PROTOCOL` and `PROJECT_DIRECTORY_PROTOCOL` (`wire/proto`).

import type { NodeKey } from "../../../wire/bridge";
import type { FolderSuggestions, ProjectInfo } from "../../../wire/proto";
import type { Fleet } from "../../fleet";
import { attempt, type Outcome } from "../../outcome";
import { addProject, cloneProject } from "../projects";

/** Paseo's rows per search. */
export const FOLDER_ROWS = 30;

/** Folders under `node`'s home matching `query`. */
export function askFolders(
  fleet: Fleet,
  node: NodeKey,
  query: string,
): Promise<Outcome<FolderSuggestions>> {
  return attempt(() =>
    fleet.bridge.call(node, "folder.suggest", { query, limit: FOLDER_ROWS }),
  );
}

/** Make the folder `name` in `parent` on `node` and add it as a project. */
export function createProjectDirectory(
  fleet: Fleet,
  node: NodeKey,
  parent: string,
  name: string,
): Promise<Outcome<ProjectInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "project.create_directory", { parent, name }),
  );
}

/** Paseo's name check before asking (`createDirectory`): empty, `.`,
 * `..` or a separator is "Enter a directory name"; the node checks the
 * rest. */
export function directoryNameProblem(name: string): string | null {
  const trimmed = name.trim();
  return trimmed === "" ||
    trimmed === "." ||
    trimmed === ".." ||
    /[\\/]/u.test(trimmed)
    ? "Enter a directory name"
    : null;
}

/** A submission's end: the project, or Paseo's words for the failure. */
export type Submitted =
  { ok: true; value: ProjectInfo } | { ok: false; message: string };

/** Add the folder `path`: Paseo's "Directory not found" for a missing
 * folder (the node's "does not exist"), else "Unable to add project". */
export async function addFolder(
  fleet: Fleet,
  node: NodeKey,
  path: string,
): Promise<Submitted> {
  const added = await addProject(fleet, node, path, null);
  if (added.ok) return added;
  return {
    ok: false,
    message: added.problem.message.includes("does not exist")
      ? "Directory not found"
      : "Unable to add project",
  };
}

/** Clone `remote` into `parent`: the node's words when it refused. */
export async function cloneInto(
  fleet: Fleet,
  node: NodeKey,
  remote: string,
  parent: string,
): Promise<Submitted> {
  const cloned = await cloneProject(fleet, node, {
    url: remote,
    parent,
    folder: null,
  });
  if (cloned.ok) return cloned;
  return {
    ok: false,
    message: wordsOr(cloned.problem.message, "Unable to clone repository"),
  };
}

/** Make `name` in `parent` a project: the node's words when it refused. */
export async function makeDirectory(
  fleet: Fleet,
  node: NodeKey,
  parent: string,
  name: string,
): Promise<Submitted> {
  const made = await createProjectDirectory(fleet, node, parent, name);
  if (made.ok) return made;
  return {
    ok: false,
    message: wordsOr(made.problem.message, "Unable to create directory"),
  };
}

function wordsOr(message: string, otherwise: string): string {
  return message === "" ? otherwise : message;
}
