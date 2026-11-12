// One row of the Changes list (data/workspace/changes.ts): a file's
// header (agent-orchestrator's review pane: 36px, a chevron, the file's
// mark, its folder muted and its name in the ink, the status letter and
// the counts, its actions on hover), a hunk's band, a line of the diff,
// or a note in place of a diff (loading, binary, too large, more).

import {
  type ChangeRow,
  type FileNote,
  splitPath,
  statusLetter,
} from "../../data/workspace/changes";
import type { DiffRow } from "../../data/workspace/diff-rows";
import { sizeWord } from "../../data/workspace/size";
import { languageOf } from "../../data/workspace/syntax-languages";
import { Icon, IconButton, Spinner, Tooltip } from "../../ui/kit";
import type { FileChange, LineKind } from "../../wire/proto";
import { CodeText } from "./code-text";

/** What a file's header offers, by the comparison it is in. */
export interface FileActions {
  /** Stage (`stage`) or unstage (`unstage`) the file, or neither. */
  index: "stage" | "unstage" | null;
  onIndex: (change: FileChange) => void;
  onDiscard: ((change: FileChange) => void) | null;
  onOpen: (change: FileChange) => void;
  onToggle: (change: FileChange) => void;
}

function lineClass(kind: LineKind): string {
  switch (kind) {
    case "added":
      return "d-line add";
    case "removed":
      return "d-line del";
    case "context":
      return "d-line";
    case "no_newline":
      return "d-line eof";
  }
}

function marker(kind: LineKind): string {
  switch (kind) {
    case "added":
      return "+";
    case "removed":
      return "-";
    case "context":
    case "no_newline":
      return "";
  }
}

function FileHeader({
  change,
  open,
  actions,
}: {
  change: FileChange;
  open: boolean;
  actions: FileActions;
}) {
  const { dir, name } = splitPath(change.path);
  const status = statusLetter(change.status);
  return (
    <div className="d-file" data-open={open ? "" : undefined}>
      <button
        type="button"
        className="d-file-toggle"
        aria-expanded={open}
        aria-label={`${open ? "Collapse" : "Expand"} ${change.path}`}
        onClick={() => {
          actions.onToggle(change);
        }}
      >
        <Icon name="chevron" size={14} />
        <Icon name="file" size={14} />
        <span className="d-path">
          {/* Cut from the left: a path's end is what tells two apart. */}
          <bdi>
            <span className="d-dir">{dir}</span>
            <span className="d-name">{name}</span>
          </bdi>
        </span>
        <span className={`d-status ${status.tone}`} aria-label={status.word}>
          {status.letter}
        </span>
        {change.binary ? null : (
          <span className="d-counts">
            <span className="d-add">+{String(change.additions ?? 0)}</span>
            <span className="d-del">−{String(change.deletions ?? 0)}</span>
          </span>
        )}
      </button>
      <span className="d-file-actions">
        {actions.index === null ? null : (
          <Tooltip label={actions.index === "stage" ? "Stage" : "Unstage"}>
            <IconButton
              label={`${actions.index === "stage" ? "Stage" : "Unstage"} ${change.path}`}
              icon={actions.index === "stage" ? "plus" : "minus"}
              size="xs"
              variant="ghost"
              onClick={() => {
                actions.onIndex(change);
              }}
            />
          </Tooltip>
        )}
        {actions.onDiscard === null ? null : (
          <Tooltip label="Discard changes">
            <IconButton
              label={`Discard changes to ${change.path}`}
              icon="undo"
              size="xs"
              variant="ghost"
              onClick={() => {
                actions.onDiscard?.(change);
              }}
            />
          </Tooltip>
        )}
        {change.status === "deleted" ? null : (
          <Tooltip label="Open full file">
            <IconButton
              label={`Open ${change.path}`}
              icon="file-code"
              size="xs"
              variant="ghost"
              onClick={() => {
                actions.onOpen(change);
              }}
            />
          </Tooltip>
        )}
      </span>
    </div>
  );
}

function noteText(note: FileNote): string {
  switch (note.kind) {
    case "loading":
      return "Loading diff…";
    case "more":
      return "Loading more of this diff…";
    case "binary":
      return "Binary file preview is not available.";
    case "too_large":
      return `This diff is too large to preview (${sizeWord(note.bytes)}). Open the full file to inspect it.`;
    case "submodule":
      return `Submodule ${note.from?.slice(0, 7) ?? "(none)"} → ${note.to?.slice(0, 7) ?? "(none)"}`;
    case "unchanged":
      return "Only the file's mode changed.";
    case "unknown":
      return "This page cannot show this kind of change.";
    case "failed":
      return `Diff unavailable: ${note.message}`;
  }
}

function DiffLineRow({ row, path }: { row: DiffRow; path: string }) {
  if (row.kind === "cut") {
    return (
      <div className="d-note">
        <span>
          This hunk is longer than the 5,000 lines shown of it; open the full
          file to read the rest.
        </span>
      </div>
    );
  }
  if (row.kind === "hunk") {
    return (
      <div className="d-hunk">
        <span>{row.header}</span>
        {row.section === null ? null : (
          <span className="d-section">{row.section}</span>
        )}
      </div>
    );
  }
  const number = row.new ?? row.old;
  return (
    <div
      className={lineClass(row.line)}
      data-path={path}
      data-kind={row.line}
      data-old={row.old ?? undefined}
      data-new={row.new ?? undefined}
    >
      <span className="d-num" aria-hidden="true">
        {number === null ? "" : String(number)}
      </span>
      <span className="d-mark" aria-hidden="true">
        {marker(row.line)}
      </span>
      <span className="d-code">
        {row.line === "no_newline" ? (
          row.text
        ) : (
          <CodeText text={row.text} language={languageOf(path)} />
        )}
        {row.cut ? <span className="d-cut"> …</span> : null}
      </span>
    </div>
  );
}

export function ChangeRowView({
  row,
  path,
  actions,
}: {
  row: ChangeRow;
  /** The file the row belongs to. */
  path: string;
  actions: FileActions;
}) {
  switch (row.kind) {
    case "file":
      return (
        <FileHeader change={row.change} open={row.open} actions={actions} />
      );
    case "note":
      return (
        <div
          className={row.note.kind === "failed" ? "d-note failed" : "d-note"}
        >
          {row.note.kind === "loading" || row.note.kind === "more" ? (
            <Spinner size={12} />
          ) : null}
          <span>{noteText(row.note)}</span>
        </div>
      );
    case "hunk":
    case "line":
    case "cut":
      return <DiffLineRow row={row} path={path} />;
  }
}
