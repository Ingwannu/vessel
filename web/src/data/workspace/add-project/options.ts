// Add project's rows that are not paths on the machine: machines, methods,
// manual GitHub choices, parents and clone destinations. Paseo's
// `add-project-flow/options.ts` (pinned 0.15.4), its host as Vessel's
// machine (docs/plans/wp-add-project-flow.md, differences 1, 9-11).

import {
  isCompleteGitRemote,
  parseGitHubRemoteUrl,
  parseGitRemoteLocation,
} from "../../git-remote";
import type { AddProjectMachine, GithubRepositoryChoice } from "./model";
import { shortenPath } from "./paths";

/** The four ways, in Paseo's order. */
export type AddProjectMethodId =
  "directory-search" | "browse" | "github" | "new-directory";

export interface AddProjectMethodOption {
  id: AddProjectMethodId;
  label: string;
  description: string;
  disabled?: boolean;
}

/** A clone destination: the parent it is cloned into, and the folder it
 * makes there. */
export interface AddProjectPathOption {
  id: string;
  path: string;
  displayPath: string;
  secondaryText: string | null;
  disabled: boolean;
}

/** The machines whose name or key holds the field's text. */
export function filterAddProjectMachines(
  machines: readonly AddProjectMachine[],
  query: string,
): AddProjectMachine[] {
  const wanted = query.trim().toLowerCase();
  if (wanted === "") return [...machines];
  return machines.filter(
    (machine) =>
      machine.label.toLowerCase().includes(wanted) ||
      machine.node.toLowerCase().includes(wanted),
  );
}

/** The methods `machine` offers. `browseIn` names the platform's file
 * manager (difference 10); only the native app offers Browse. */
export function buildAddProjectMethods(
  machine: AddProjectMachine,
  browseIn = "the file manager",
): AddProjectMethodOption[] {
  if (!machine.canAddProject) return [];
  const options: AddProjectMethodOption[] = [
    {
      id: "directory-search",
      label: "Search for directory",
      description: `Find a directory on ${machine.label}`,
    },
  ];
  if (machine.canBrowse) {
    options.push({
      id: "browse",
      label: "Browse",
      description: `Choose or create a directory in ${browseIn}`,
    });
  }
  options.push({
    id: "github",
    label: "Clone from GitHub",
    description: githubMethodDescription(machine),
    disabled: !machine.canCloneGithubRepositories,
  });
  options.push({
    id: "new-directory",
    label: "New directory",
    description: machine.canCreateDirectory
      ? `Create an empty directory on ${machine.label}`
      : "Update this machine to create directories",
    disabled: !machine.canCreateDirectory,
  });
  return options;
}

/** The method page's text when it has no rows. */
export function addProjectMethodEmptyText(
  machine: AddProjectMachine | null,
): string {
  return machine?.canAddProject === false
    ? "Update the machine to use Add Project."
    : "No matching options";
}

function githubMethodDescription(machine: AddProjectMachine): string {
  if (!machine.canCloneGithubRepositories) {
    return "Update this machine to clone GitHub repositories";
  }
  if (machine.canSearchGithubRepositories) {
    return "Search projects available to your GitHub account";
  }
  return "Enter a GitHub URL or owner/repo";
}

/** The last segment of a path. */
export function pathBaseName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/u, "");
  return trimmed.split(/[\\/]/u).at(-1) ?? trimmed;
}

const SHORTHAND = /^([^\s/]+)\/([^\s/]+)$/u;

/** Rows for what was typed: a complete remote as itself, `owner/repo`
 * via HTTPS and via SSH, else none. */
export function buildManualGithubRepositoryChoices(
  query: string,
): GithubRepositoryChoice[] {
  const repo = query.trim();
  if (repo === "") return [];
  if (isCompleteGitRemote(repo)) {
    const identity = parseGitHubRemoteUrl(repo);
    const location = parseGitRemoteLocation(repo);
    const remoteName =
      location === null
        ? repo
        : pathBaseName(location.path).replace(/\.git$/u, "");
    return [
      {
        id: `manual:${repo}`,
        nameWithOwner: identity?.repo ?? remoteName,
        cloneUrl: repo,
        description: "Clone this repository URL",
        updatedAt: null,
      },
    ];
  }
  const shorthand = SHORTHAND.exec(repo);
  if (shorthand === null) return [];
  const nameWithOwner = `${shorthand[1] ?? ""}/${shorthand[2] ?? ""}`;
  return (["https", "ssh"] as const).map((cloneProtocol) => ({
    id: `manual:${cloneProtocol}:${nameWithOwner}`,
    nameWithOwner,
    cloneUrl: nameWithOwner,
    cloneProtocol,
    description: `Clone owner/repo via ${cloneProtocol.toUpperCase()}`,
    updatedAt: null,
  }));
}

/** The remote `project.clone` fetches for a choice. Paseo's server turns
 * `owner/repo` and a protocol into a GitHub remote; Vessel's node takes a
 * URL, so the client does (difference 11). */
export function cloneRemote(choice: GithubRepositoryChoice): string {
  if (choice.cloneProtocol === "https") {
    return `https://github.com/${choice.nameWithOwner}.git`;
  }
  if (choice.cloneProtocol === "ssh") {
    return `git@github.com:${choice.nameWithOwner}.git`;
  }
  return choice.cloneUrl;
}

/** The folder a path is in; `null` for a bare name. */
export function parentDirectory(path: string): string | null {
  const trimmed = path.replace(/[\\/]+$/u, "");
  const index = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  if (index < 0) return null;
  if (index === 0) return trimmed.slice(0, 1);
  return trimmed.slice(0, index);
}

/** `name` in `parent`, with the parent's own separator. */
export function joinDirectoryPath(parent: string, name: string): string {
  const trimmed = parent.replace(/[\\/]+$/u, "");
  const separator =
    trimmed.includes("\\") && !trimmed.includes("/") ? "\\" : "/";
  return `${trimmed}${separator}${name}`;
}

/** Where a clone or a new directory may go: the folders the machine's
 * projects are in, then Paseo's usual places, without repeats. */
export function buildSuggestedParentDirectories(
  projectPaths: readonly string[],
): string[] {
  const values = [
    ...projectPaths.flatMap((path) => {
      const parent = parentDirectory(path);
      return parent === null ? [] : [parent];
    }),
    "~/dev",
    "~/Developer",
    "~/src",
    "~/projects",
    "~/workspace",
    "~",
  ];
  return [...new Set(values)];
}

/** A path's identity for comparing: the home as `~`, `/` separators, no
 * trailing separator; a Windows path in lower case. */
function pathIdentity(path: string): string {
  const normalized = shortenPath(path.trim())
    .replace(/\\/gu, "/")
    .replace(/\/+$/u, "");
  return /^[A-Za-z]:\//u.test(normalized) || normalized.startsWith("//")
    ? normalized.toLowerCase()
    : normalized;
}

/** Each parent once (a home path and its `~` form are one), with the
 * folder the clone would make there; one that exists is disabled. */
export function buildCloneLocationOptions(input: {
  parents: readonly string[];
  repositoryName: string;
  existingPaths: readonly string[];
}): AddProjectPathOption[] {
  const existing = new Set(input.existingPaths.map(pathIdentity));
  const seen = new Set<string>();
  return input.parents.flatMap((parent) => {
    const path = joinDirectoryPath(parent, input.repositoryName);
    const identity = pathIdentity(path);
    if (seen.has(identity)) return [];
    seen.add(identity);
    const exists = existing.has(identity);
    return [
      {
        id: parent,
        path: parent,
        displayPath: path,
        secondaryText: exists
          ? "Already exists"
          : `Parent directory: ${parent}`,
        disabled: exists,
      },
    ];
  });
}
