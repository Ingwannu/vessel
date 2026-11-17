// The disk moved under an open file (Paseo's `FileConflictAlert`,
// `file-panels/source/conflict-alert.tsx`; ADR 0105): under the file's
// header, an amber title, Paseo's description, and its choices.
//
// [Decision Log]
// - Purpose: an edit is never replaced under the person, and the page says
//   at once that the file changed or went, with the way out beside it.
// - Constraint: the page edits on Edit (Paseo's editor is always on), so
//   "modified" is an open edit whose text differs from what was read;
//   Overwrite is the save's forced write, Reload drops the edit and reads
//   the file again.
// - Choice: Paseo's states and words: changed (Overwrite only while the
//   edit holds changes; Reload, which asks first when it would throw them
//   away), and deleted (no choices: the open copy stays). A save into a
//   changed file still meets the two-sided conflict (`file-conflict.tsx`).
// - Cost: a second place that says "changed on disk"; the save's conflict
//   replaces this alert while it is shown, so the two never stack.

import "./file-change-alert.css";

import { Button, Icon } from "../../ui/kit";

/** What the alert says (Paseo's `FileConflictAlertState`). */
export type DiskChange =
  | {
      kind: "changed";
      /** The edit holds changes of its own: they can be written over the
       * disk's. */
      canOverwrite: boolean;
      busy: boolean;
      onReload: () => void;
      onOverwrite: () => void;
    }
  | { kind: "deleted" };

export function FileChangeAlert({ change }: { change: DiskChange }) {
  const title =
    change.kind === "deleted" ? "File deleted on disk" : "Changed on disk";
  const description =
    change.kind === "deleted"
      ? "The open copy is preserved."
      : change.canOverwrite
        ? "The local buffer was preserved. Choose which version to keep."
        : null;
  return (
    <div className="file-change-alert" role="alert">
      <Icon name="warn" size={16} />
      <div className="file-change-message">
        <span className="file-change-title">{title}</span>
        {description === null ? null : (
          <span className="file-change-description">{description}</span>
        )}
      </div>
      {change.kind === "changed" ? (
        <div className="file-change-actions">
          {change.canOverwrite ? (
            <Button
              variant="outline"
              size="sm"
              busy={change.busy}
              onClick={change.onOverwrite}
            >
              Overwrite
            </Button>
          ) : null}
          <Button
            variant="outline"
            size="sm"
            disabled={change.busy}
            onClick={change.onReload}
          >
            Reload
          </Button>
        </div>
      ) : null}
    </div>
  );
}
