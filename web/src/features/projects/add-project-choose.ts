// What choosing a row of the Add project sheet does (TRACKER 234),
// Paseo's `add-project-flow.tsx` row `select`s and `createDirectory`:
// the next page, or a call (`use-add-project-submit.ts`).

import { openAddMachine } from "../../data/add-machine";
import type { Fleet } from "../../data/fleet";
import {
  addFolder,
  cloneInto,
  directoryNameProblem,
  makeDirectory,
} from "../../data/workspace/add-project/calls";
import { setAddProjectPageStatus } from "../../data/workspace/add-project/editing";
import {
  type AddProjectFlowState,
  type AddProjectPage,
  chooseAddProjectMachine,
  openDirectorySearchPage,
  openGithubLocationPage,
  openGithubSearchPage,
  openNewDirectoryNamePage,
  openNewDirectoryParentPage,
} from "../../data/workspace/add-project/model";
import { cloneRemote } from "../../data/workspace/add-project/options";
import type { FlowRow } from "../../data/workspace/add-project/rows";
import type { NodeKey } from "../../wire/bridge";
import type { Submit } from "./use-add-project-submit";

/** What a choice acts on: the sheet's page and its machine. */
export interface ChoiceContext {
  fleet: Fleet;
  node: NodeKey | null;
  page: AddProjectPage;
  setState: (
    update: (current: AddProjectFlowState) => AddProjectFlowState,
  ) => void;
  submit: Submit;
  onClose: () => void;
  /** Where the last clone on each machine went. */
  lastCloneParent: Map<NodeKey, string>;
}

export function chooseRow(row: FlowRow, context: ChoiceContext): void {
  const { fleet, node, page, setState, submit } = context;
  const action = row.action;
  if (row.disabled) return;
  if (action.kind === "machine") {
    setState((current) => chooseAddProjectMachine(current, action.node));
    return;
  }
  if (action.kind === "add-machine") {
    context.onClose();
    openAddMachine();
    return;
  }
  if (node === null) return;
  switch (action.kind) {
    case "method":
      if (action.method === "directory-search") {
        setState((current) => openDirectorySearchPage(current, node));
      } else if (action.method === "github") {
        setState((current) => openGithubSearchPage(current, node));
      } else if (action.method === "new-directory") {
        setState((current) => openNewDirectoryParentPage(current, node));
      }
      return;
    case "add-path":
      submit("directory-search", () => addFolder(fleet, node, action.path));
      return;
    case "repository":
      setState((current) =>
        openGithubLocationPage(current, node, action.repository),
      );
      return;
    case "clone":
      if (page.kind !== "github-location") return;
      submit("github-location", async () => {
        const remote = cloneRemote(page.repository);
        const cloned = await cloneInto(fleet, node, remote, action.parent);
        if (cloned.ok) context.lastCloneParent.set(node, action.parent);
        return cloned;
      });
      return;
    case "parent":
      setState((current) =>
        openNewDirectoryNamePage(current, node, action.path),
      );
      return;
  }
}

/** The name page's Enter: Paseo's name check, then the call. */
export function createDirectory(context: ChoiceContext): void {
  const { fleet, node, page, setState, submit } = context;
  if (page.kind !== "new-directory-name" || node === null) return;
  const problem = directoryNameProblem(page.name);
  if (problem !== null) {
    setState((current) =>
      setAddProjectPageStatus(current, "new-directory-name", {
        error: problem,
      }),
    );
    return;
  }
  const { parentPath } = page;
  const name = page.name.trim();
  submit("new-directory-name", () =>
    makeDirectory(fleet, node, parentPath, name),
  );
}
