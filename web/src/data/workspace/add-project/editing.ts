// Add project's edits to the page shown: its field (with the drafts kept
// across Back), its active row, its progress and error, and the keyboard's
// wrap-and-skip move. Paseo's `add-project-flow/model.ts` (pinned 0.15.4),
// the part after its page stack (`model.ts`).

import {
  type AddProjectFlowState,
  type AddProjectPage,
  currentAddProjectPage,
  draftKey,
  updateCurrentPage,
} from "./model";

/** The field changed: the page's text, its first row active, no error;
 * a destination page keeps it as its draft. */
export function setAddProjectPageInput(
  state: AddProjectFlowState,
  value: string,
): AddProjectFlowState {
  const page = currentAddProjectPage(state);
  const updated = updateCurrentPage(state, (current) => {
    if (current.kind === "new-directory-name") {
      return { ...current, name: value, activeIndex: 0, error: null };
    }
    if (current.kind === "method") return current;
    return { ...current, query: value, activeIndex: 0, error: null };
  });
  if (page.kind !== "github-location") return updated;
  return {
    ...updated,
    githubLocationDrafts: {
      ...updated.githubLocationDrafts,
      [draftKey(page.node, page.repository.id)]: {
        query: value,
        activeIndex: 0,
      },
    },
  };
}

/** The name page's field changed, kept as the draft for its parent. */
export function setNewDirectoryName(
  state: AddProjectFlowState,
  value: string,
): AddProjectFlowState {
  const page = currentAddProjectPage(state);
  if (page.kind !== "new-directory-name") return state;
  const updated = setAddProjectPageInput(state, value);
  return {
    ...updated,
    newDirectoryNameDrafts: {
      ...updated.newDirectoryNameDrafts,
      [draftKey(page.node, page.parentPath)]: value,
    },
  };
}

export function setAddProjectActiveIndex(
  state: AddProjectFlowState,
  activeIndex: number,
): AddProjectFlowState {
  const page = currentAddProjectPage(state);
  const updated = updateCurrentPage(state, (current) => ({
    ...current,
    activeIndex,
  }));
  if (page.kind !== "github-location") return updated;
  return {
    ...updated,
    githubLocationDrafts: {
      ...updated.githubLocationDrafts,
      [draftKey(page.node, page.repository.id)]: {
        query: page.query,
        activeIndex,
      },
    },
  };
}

/** The page's progress and error, when it is still the page shown. */
export function setAddProjectPageStatus(
  state: AddProjectFlowState,
  kind: AddProjectPage["kind"],
  status: { isSubmitting?: boolean; error?: string | null },
): AddProjectFlowState {
  return updateCurrentPage(state, (page) =>
    page.kind === kind ? { ...page, ...status } : page,
  );
}

/** The next or previous of `count` rows, wrapping. */
export function moveAddProjectActiveIndex(
  activeIndex: number,
  count: number,
  direction: "next" | "previous",
): number {
  if (count === 0) return 0;
  const next = activeIndex + (direction === "next" ? 1 : -1);
  if (next < 0) return count - 1;
  if (next >= count) return 0;
  return next;
}

/** The next or previous row that can be chosen, wrapping; where it was
 * when none can. */
export function moveAddProjectSelection(
  activeIndex: number,
  selectable: readonly boolean[],
  direction: "next" | "previous",
): number {
  if (!selectable.some(Boolean)) return 0;
  let next = activeIndex;
  let left = selectable.length;
  while (left > 0) {
    left -= 1;
    next = moveAddProjectActiveIndex(next, selectable.length, direction);
    if (selectable[next] === true) return next;
  }
  return activeIndex;
}
