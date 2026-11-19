// Add project's page stack, Paseo's `add-project-flow/model.ts` (pinned
// 0.15.4) with its host as Vessel's machine (TRACKER 234-238).
//
// [Decision Log]
// - Purpose: the whole Add project flow as Paseo's sheet: where Paseo's
//   behaviour lives (the back stack, the drafts that survive Back, the
//   wrap-and-skip keyboard moves) as plain state, tested without drawing.
// - Constraint: the native app draws the same flow from its own port
//   (`view::workspace::add_flow`); both follow the same reference and
//   tests (docs/plans/wp-add-project-flow.md).
// - Choice: Paseo's state and functions one for one, each returning a new
//   state; the sheet (`features/projects/add-project-flow.tsx`) holds one
//   and draws its last page.
// - Cost: the GitHub pages are here before the sheet draws search rows
//   (TRACKER 440); the model is the same with or without them.

import type { NodeKey } from "../../../wire/bridge";

/** A machine the sheet may add to, and which ways it offers. */
export interface AddProjectMachine {
  node: NodeKey;
  label: string;
  canAddProject: boolean;
  canBrowse: boolean;
  canCloneGithubRepositories: boolean;
  canSearchGithubRepositories: boolean;
  canCreateDirectory: boolean;
}

/** A repository to clone: a search result, or one typed by hand. */
export interface GithubRepositoryChoice {
  id: string;
  nameWithOwner: string;
  cloneUrl: string;
  cloneProtocol?: "https" | "ssh";
  description: string | null;
  updatedAt: string | null;
}

interface PageState {
  activeIndex: number;
  error: string | null;
}

interface SearchPageState extends PageState {
  query: string;
}

/** One page of the sheet. */
export type AddProjectPage =
  | ({ kind: "host" } & SearchPageState)
  | ({ kind: "method"; node: NodeKey; isSubmitting: boolean } & PageState)
  | ({
      kind: "directory-search";
      node: NodeKey;
      isSubmitting: boolean;
    } & SearchPageState)
  | ({ kind: "github-search"; node: NodeKey } & SearchPageState)
  | ({
      kind: "github-location";
      node: NodeKey;
      repository: GithubRepositoryChoice;
      isSubmitting: boolean;
    } & SearchPageState)
  | ({ kind: "new-directory-parent"; node: NodeKey } & SearchPageState)
  | {
      kind: "new-directory-name";
      node: NodeKey;
      parentPath: string;
      name: string;
      activeIndex: number;
      error: string | null;
      isSubmitting: boolean;
    };

/** The sheet's state: its machines, its pages (the last is shown) and the
 * drafts kept across Back. */
export interface AddProjectFlowState {
  machines: AddProjectMachine[];
  pages: AddProjectPage[];
  newDirectoryNameDrafts: Readonly<Record<string, string>>;
  githubLocationDrafts: Readonly<
    Record<string, { query: string; activeIndex: number }>
  >;
}

type SearchPageKind = Extract<AddProjectPage, { query: string }>["kind"];

function searchPage<Kind extends SearchPageKind>(
  kind: Kind,
): { kind: Kind; query: string; activeIndex: number; error: null } {
  return { kind, query: "", activeIndex: 0, error: null };
}

function methodPage(node: NodeKey): AddProjectPage {
  return {
    kind: "method",
    node,
    activeIndex: 0,
    error: null,
    isSubmitting: false,
  };
}

/** The machine the sheet starts on: the preferred one, else the only one. */
function initialMachine(
  machines: readonly AddProjectMachine[],
  preferred: NodeKey | undefined,
): AddProjectMachine | null {
  const chosen =
    preferred === undefined
      ? undefined
      : machines.find((machine) => machine.node === preferred);
  if (chosen !== undefined) return chosen;
  return machines.length === 1 ? (machines[0] ?? null) : null;
}

/** A new sheet: on the preferred or only machine's methods, else on the
 * machine page (so Back from the methods closes). */
export function openAddProjectFlow(input: {
  machines: AddProjectMachine[];
  preferred?: NodeKey;
}): AddProjectFlowState {
  const initial = initialMachine(input.machines, input.preferred);
  return {
    machines: input.machines,
    pages: initial === null ? [searchPage("host")] : [methodPage(initial.node)],
    newDirectoryNameDrafts: {},
    githubLocationDrafts: {},
  };
}

/** Machines arrived or changed: while the machine page is the only page,
 * a preferred or only machine skips it. */
export function applyAvailableMachines(
  state: AddProjectFlowState,
  machines: AddProjectMachine[],
  preferred?: NodeKey,
): AddProjectFlowState {
  const current = currentAddProjectPage(state);
  if (state.pages.length !== 1 || current.kind !== "host") {
    return { ...state, machines };
  }
  const initial = initialMachine(machines, preferred);
  return {
    ...state,
    machines,
    pages: initial === null ? state.pages : [methodPage(initial.node)],
  };
}

/** The page shown. */
export function currentAddProjectPage(
  state: AddProjectFlowState,
): AddProjectPage {
  const page = state.pages.at(-1);
  if (page === undefined) {
    throw new Error("The Add project sheet always has a page");
  }
  return page;
}

/** `state` with its last page changed by `update`. */
export function updateCurrentPage(
  state: AddProjectFlowState,
  update: (page: AddProjectPage) => AddProjectPage,
): AddProjectFlowState {
  const last = state.pages.length - 1;
  return {
    ...state,
    pages: state.pages.map((page, index) =>
      index === last ? update(page) : page,
    ),
  };
}

function pushPage(
  state: AddProjectFlowState,
  page: AddProjectPage,
): AddProjectFlowState {
  return { ...state, pages: [...state.pages, page] };
}

/** One page back; `null` on the first page (the sheet closes). */
export function backAddProjectPage(
  state: AddProjectFlowState,
): AddProjectFlowState | null {
  if (state.pages.length === 1) return null;
  return { ...state, pages: state.pages.slice(0, -1) };
}

export function chooseAddProjectMachine(
  state: AddProjectFlowState,
  node: NodeKey,
): AddProjectFlowState {
  return pushPage(state, methodPage(node));
}

export function openDirectorySearchPage(
  state: AddProjectFlowState,
  node: NodeKey,
): AddProjectFlowState {
  return pushPage(state, {
    ...searchPage("directory-search"),
    node,
    isSubmitting: false,
  });
}

export function openGithubSearchPage(
  state: AddProjectFlowState,
  node: NodeKey,
): AddProjectFlowState {
  return pushPage(state, { ...searchPage("github-search"), node });
}

/** A draft's key: the machine and the page's parent or repository. */
export function draftKey(node: NodeKey, other: string): string {
  return `${node}\u0000${other}`;
}

/** The destination page for `repository`, with what was typed there the
 * last time it was open. */
export function openGithubLocationPage(
  state: AddProjectFlowState,
  node: NodeKey,
  repository: GithubRepositoryChoice,
): AddProjectFlowState {
  const draft = state.githubLocationDrafts[draftKey(node, repository.id)];
  return pushPage(state, {
    kind: "github-location",
    query: draft?.query ?? "",
    activeIndex: draft?.activeIndex ?? 0,
    error: null,
    node,
    repository,
    isSubmitting: false,
  });
}

export function openNewDirectoryParentPage(
  state: AddProjectFlowState,
  node: NodeKey,
): AddProjectFlowState {
  return pushPage(state, { ...searchPage("new-directory-parent"), node });
}

/** The name page under `parentPath`, with the name typed there before. */
export function openNewDirectoryNamePage(
  state: AddProjectFlowState,
  node: NodeKey,
  parentPath: string,
): AddProjectFlowState {
  return pushPage(state, {
    kind: "new-directory-name",
    node,
    parentPath,
    name: state.newDirectoryNameDrafts[draftKey(node, parentPath)] ?? "",
    activeIndex: 0,
    error: null,
    isSubmitting: false,
  });
}
