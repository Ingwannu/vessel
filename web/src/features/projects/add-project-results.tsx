// What the Add project sheet lists under its field (TRACKER 234), in
// Paseo's order (`add-project-flow.tsx`, pinned 0.15.4): the new
// directory's path, the progress while a call runs (nothing else then),
// the page's error, the search's error, "Loading...", the rows, and the
// page's empty words.

import type { ReactElement, Ref } from "react";

import type {
  AddProjectMachine,
  AddProjectPage,
} from "../../data/workspace/add-project/model";
import { joinDirectoryPath } from "../../data/workspace/add-project/options";
import { shortenPath } from "../../data/workspace/add-project/paths";
import {
  emptyText,
  type FlowRow,
  progressText,
} from "../../data/workspace/add-project/rows";
import { Icon, ProviderMark } from "../../ui/kit";

function RowMark({ row }: { row: FlowRow }): ReactElement {
  // GitHub's brand mark; no hard-drive icon here, so a destination's is
  // the folder (plan difference 12).
  if (row.mark === "github") return <ProviderMark name="github" size={16} />;
  if (row.mark === "drive") return <Icon name="folder" size={16} />;
  return <Icon name={row.mark} size={16} />;
}

export function AddProjectResults({
  page,
  machine,
  rows,
  activeIndex,
  activeRef,
  loading,
  queryError,
  onChoose,
}: {
  page: AddProjectPage;
  machine: AddProjectMachine | null;
  rows: readonly FlowRow[];
  activeIndex: number;
  activeRef: Ref<HTMLButtonElement>;
  loading: boolean;
  queryError: string | null;
  onChoose: (row: FlowRow) => void;
}): ReactElement {
  const submitting = "isSubmitting" in page && page.isSubmitting;
  const preview =
    page.kind === "new-directory-name" && page.name.trim() !== ""
      ? joinDirectoryPath(page.parentPath, page.name.trim())
      : null;
  const showRows = !submitting && !loading && queryError === null;
  return (
    <>
      {preview === null ? null : (
        <p className="add-project-preview">{shortenPath(preview)}</p>
      )}
      {submitting ? (
        <p className="add-project-state">{progressText(page)}</p>
      ) : null}
      {!submitting && page.error !== null ? (
        <p className="add-project-error">{page.error}</p>
      ) : null}
      {!submitting && queryError !== null ? (
        <p className="add-project-error">{queryError}</p>
      ) : null}
      {!submitting && loading ? (
        <p className="add-project-state">Loading...</p>
      ) : null}
      {showRows
        ? rows.map((row, index) => (
            <button
              key={row.id}
              ref={index === activeIndex ? activeRef : undefined}
              type="button"
              className="add-project-row"
              data-active={index === activeIndex ? "" : undefined}
              disabled={row.disabled}
              onClick={() => {
                onChoose(row);
              }}
            >
              <span className="add-project-mark" aria-hidden="true">
                <RowMark row={row} />
              </span>
              <span className="add-project-text">
                <span className="add-project-row-title">{row.title}</span>
                {row.subtitle === null ? null : (
                  <span className="add-project-row-subtitle">
                    {row.subtitle}
                  </span>
                )}
              </span>
            </button>
          ))
        : null}
      {showRows && rows.length === 0 && page.kind !== "new-directory-name" ? (
        <p className="add-project-state">{emptyText(page, machine)}</p>
      ) : null}
    </>
  );
}
