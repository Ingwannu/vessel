// What each page of the Add project sheet lists and says, Paseo's
// `add-project-flow.tsx` rows, titles, placeholders and state words
// (pinned 0.15.4), host as machine. Pure: the sheet
// (`features/projects/add-project-flow.tsx`) feeds it what it read and
// acts on the chosen row's action.

import type {
  AddProjectMachine,
  AddProjectPage,
  GithubRepositoryChoice,
} from "./model";
import {
  type AddProjectMethodId,
  addProjectMethodEmptyText,
  buildAddProjectMethods,
  buildCloneLocationOptions,
  buildManualGithubRepositoryChoices,
  buildSuggestedParentDirectories,
  filterAddProjectMachines,
  pathBaseName,
} from "./options";
import {
  buildDirectoryOptions,
  type DirectoryOption,
  shortenPath,
} from "./paths";

/** The mark a row draws (Paseo's lucide icons; `drive` is its HardDrive). */
export type RowMark =
  | "machine"
  | "plus"
  | "search"
  | "folder-open"
  | "github"
  | "folder-plus"
  | "folder"
  | "drive";

/** What choosing a row does. */
export type RowAction =
  | { kind: "machine"; node: AddProjectMachine["node"] }
  | { kind: "add-machine" }
  | { kind: "method"; method: AddProjectMethodId }
  | { kind: "add-path"; path: string }
  | { kind: "repository"; repository: GithubRepositoryChoice }
  | { kind: "clone"; parent: string }
  | { kind: "parent"; path: string };

export interface FlowRow {
  id: string;
  title: string;
  subtitle: string | null;
  mark: RowMark;
  disabled: boolean;
  action: RowAction;
}

/** What the sheet knows for the page shown. */
export interface RowInputs {
  page: AddProjectPage;
  machines: readonly AddProjectMachine[];
  machine: AddProjectMachine | null;
  /** The machine's projects' roots (plan difference 4). */
  recentPaths: readonly string[];
  /** The machine's `folder.suggest` answer for the field as it is. */
  machinePaths: readonly string[];
  /** The node's `~` form of each answered path. */
  shown: ReadonlyMap<string, string>;
  /** Where the last clone on this machine went. */
  lastCloneParent: string | null;
  /** Whether the machine reads a leading `~` (ADR 0113); an older one is
   * sent no `~` path. */
  tilde: boolean;
  /** The platform's file manager, for Browse's words. */
  browseIn?: string;
}

const METHOD_MARKS: Readonly<Record<AddProjectMethodId, RowMark>> = {
  "directory-search": "search",
  browse: "folder-open",
  github: "github",
  "new-directory": "folder-plus",
};

function usable(options: DirectoryOption[], tilde: boolean): DirectoryOption[] {
  return tilde
    ? options
    : options.filter((option) => !option.path.startsWith("~"));
}

/** The rows of the page shown. */
export function buildRows(input: RowInputs): FlowRow[] {
  const { page } = input;
  const display = (path: string): string =>
    input.shown.get(path) ?? shortenPath(path);
  switch (page.kind) {
    case "host": {
      const rows = filterAddProjectMachines(input.machines, page.query).map(
        (machine): FlowRow => ({
          id: machine.node,
          title: machine.label,
          subtitle: machine.node,
          mark: "machine",
          disabled: false,
          action: { kind: "machine", node: machine.node },
        }),
      );
      if (input.machines.length === 0) {
        rows.push({
          id: "add-machine",
          title: "Add a machine",
          subtitle: "No connected machines",
          mark: "plus",
          disabled: false,
          action: { kind: "add-machine" },
        });
      }
      return rows;
    }
    case "method":
      if (input.machine === null) return [];
      return buildAddProjectMethods(input.machine, input.browseIn).map(
        (method) => ({
          id: method.id,
          title: method.label,
          subtitle: method.description,
          mark: METHOD_MARKS[method.id],
          disabled: method.disabled === true,
          action: { kind: "method", method: method.id },
        }),
      );
    case "directory-search":
      return usable(
        buildDirectoryOptions({
          recentPaths: input.recentPaths,
          machinePaths: input.machinePaths,
          query: page.query,
        }),
        input.tilde,
      ).map((option) => {
        const short = display(option.path);
        return {
          id: option.path,
          title: short,
          subtitle:
            option.kind === "path"
              ? "Open this path"
              : short === option.path
                ? null
                : option.path,
          mark: "folder",
          disabled: false,
          action: { kind: "add-path", path: option.path },
        };
      });
    case "github-search":
      return buildManualGithubRepositoryChoices(page.query).map(
        (repository) => ({
          id: repository.id,
          title:
            repository.cloneProtocol === undefined
              ? repository.nameWithOwner
              : `${repository.nameWithOwner} via ${repository.cloneProtocol.toUpperCase()}`,
          subtitle: repository.description,
          mark: "github",
          disabled: false,
          action: { kind: "repository", repository },
        }),
      );
    case "github-location": {
      const parents = buildSuggestedParentDirectories(input.recentPaths);
      const last = input.lastCloneParent;
      const ordered =
        last === null
          ? parents
          : [last, ...parents.filter((parent) => parent !== last)];
      const filtered = usable(
        buildDirectoryOptions({
          recentPaths: ordered,
          machinePaths: input.machinePaths,
          query: page.query,
        }),
        input.tilde,
      ).map((option) => option.path);
      return buildCloneLocationOptions({
        parents: filtered,
        repositoryName: pathBaseName(page.repository.nameWithOwner),
        existingPaths: [...input.recentPaths, ...input.machinePaths],
      }).map((option) => ({
        id: option.id,
        title: display(option.displayPath),
        subtitle: option.secondaryText,
        mark: "drive",
        disabled: option.disabled,
        action: { kind: "clone", parent: option.path },
      }));
    }
    case "new-directory-parent":
      return usable(
        buildDirectoryOptions({
          recentPaths: input.recentPaths,
          machinePaths: input.machinePaths,
          query: page.query,
        }),
        input.tilde,
      ).map((option) => ({
        id: option.path,
        title: display(option.path),
        subtitle: option.kind === "path" ? "Use this parent" : option.path,
        mark: "folder",
        disabled: false,
        action: { kind: "parent", path: option.path },
      }));
    case "new-directory-name":
      return [];
  }
}

/** Whether the page's field asks the machine for folders. */
export function searchesDirectories(page: AddProjectPage): boolean {
  return (
    page.kind === "directory-search" ||
    page.kind === "github-location" ||
    page.kind === "new-directory-parent"
  );
}

export function pageTitle(page: AddProjectPage): string {
  switch (page.kind) {
    case "host":
      return "Choose machine";
    case "method":
      return "Add project";
    case "directory-search":
      return "Search for directory";
    case "github-search":
      return "Clone from GitHub";
    case "github-location":
      return "Choose destination";
    case "new-directory-parent":
      return "Choose parent directory";
    case "new-directory-name":
      return "Name directory";
  }
}

/** The field's placeholder; the method page has no field. */
export function pagePlaceholder(page: AddProjectPage): string | null {
  switch (page.kind) {
    case "host":
      return "Search machines...";
    case "method":
      return null;
    case "directory-search":
      return "Search directories or enter a path...";
    case "github-search":
      return "Search or enter a GitHub repository...";
    case "github-location":
    case "new-directory-parent":
      return "Search parent directories or enter a path...";
    case "new-directory-name":
      return "Directory name";
  }
}

/** What the field holds on the page. */
export function pageInput(page: AddProjectPage): string {
  if (page.kind === "method") return "";
  return page.kind === "new-directory-name" ? page.name : page.query;
}

/** The words while the page's action runs. */
export function progressText(page: AddProjectPage): string {
  if (page.kind === "github-location") return "Cloning project...";
  if (page.kind === "new-directory-name") return "Creating directory...";
  return "Adding project...";
}

/** The words when the page lists nothing. */
export function emptyText(
  page: AddProjectPage,
  machine: AddProjectMachine | null,
): string {
  if (page.kind === "host") return "No connected machines";
  if (page.kind === "github-search") return "Enter a GitHub URL or owner/repo";
  if (page.kind === "method") return addProjectMethodEmptyText(machine);
  return "No matching options";
}
