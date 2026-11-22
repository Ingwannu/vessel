// The head of a file in the Files tab (agent-orchestrator's file pane): 36px,
// the way back, the file's path (its folders muted, its name in the ink), a
// dot while there are unsaved edits; at the right Paseo's Preview|Source
// control for a file with a Preview (`file-panels/source/toolbar.tsx`),
// then Edit, or Cancel and Save while editing.

import { useState } from "react";

import { splitPath } from "../../data/workspace/changes";
import { Button, Icon, IconButton, Segmented, Tooltip } from "../../ui/kit";
import type { RelPath } from "../../wire/proto";
import type { FileCursor } from "./file-editor";

/** Which side of a file with a Preview is shown. */
export type FileSide = "preview" | "source";

/** The side a file with a Preview shows: Preview, or Source when it was
 * opened at a line; each later line asked for turns to Source again
 * (Paseo's `useTextFileView`). */
export function useSide(
  goTo: { seq: number } | null,
): [FileSide, (side: FileSide) => void] {
  const [side, setSide] = useState<FileSide>(
    goTo === null ? "preview" : "source",
  );
  const [shownGoTo, setShownGoTo] = useState(goTo?.seq ?? null);
  if (goTo !== null && goTo.seq !== shownGoTo) {
    setShownGoTo(goTo.seq);
    setSide("source");
  }
  return [side, setSide];
}

const SIDES = [
  { value: "preview", label: "Preview" },
  { value: "source", label: "Source" },
] as const;

export function FileHead({
  path,
  dirty,
  editing,
  saving,
  cursor,
  vimMode,
  edit,
  side,
  onSide,
  onBack,
  onCancel,
  onSave,
}: {
  path: RelPath;
  dirty: boolean;
  editing: boolean;
  saving: boolean;
  cursor: FileCursor | null;
  vimMode: string | null;
  /** Edit: absent where the file cannot be edited here, `null` when it is
   * too large to. */
  edit: (() => void) | null | undefined;
  /** The side shown, for a file with a Preview. */
  side: FileSide | null;
  onSide: (side: FileSide) => void;
  onBack: () => void;
  onCancel: () => void;
  onSave: () => void;
}) {
  const { dir, name } = splitPath(path);
  return (
    <div className={editing ? "file-head editing" : "file-head"}>
      <Tooltip label="All files">
        <IconButton
          label="All files"
          icon="back"
          size="xs"
          variant="ghost"
          onClick={onBack}
        />
      </Tooltip>
      <Icon name="file" size={14} />
      <span className="file-crumb">
        <bdi>
          <span className="d-dir">{dir}</span>
          <span className="d-name">{name}</span>
        </bdi>
      </span>
      {dirty ? (
        <span className="file-dirty" aria-label="Unsaved changes" />
      ) : null}
      {vimMode !== null || cursor !== null ? (
        <span className="file-head-position">
          {vimMode === null ? null : (
            <span role="status" aria-label={`Vim mode ${vimMode}`}>
              {vimMode}
            </span>
          )}
          {cursor === null ? null : (
            <span>
              Ln {cursor.line}, Col {cursor.column}
            </span>
          )}
        </span>
      ) : null}
      <span className="file-head-end">
        {side === null ? null : (
          <Segmented
            label="File view"
            size="sm"
            value={side}
            onChange={onSide}
            options={SIDES}
          />
        )}
        {editing ? (
          <>
            <Button size="xs" variant="ghost" icon="close" onClick={onCancel}>
              Cancel
            </Button>
            <Button
              size="xs"
              variant="primary"
              busy={saving}
              disabled={!dirty}
              onClick={onSave}
            >
              Save
            </Button>
          </>
        ) : edit === undefined ? null : (
          <Tooltip label={edit === null ? "File is too large to edit" : "Edit"}>
            <IconButton
              label="Edit"
              icon="edit"
              size="xs"
              variant="ghost"
              disabled={edit === null}
              onClick={edit ?? undefined}
            />
          </Tooltip>
        )}
      </span>
    </div>
  );
}
