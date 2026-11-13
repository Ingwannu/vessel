// Every changed file's diff, one under another, in one windowed list
// (the timeline's `useWindow`, read from its top): only the rows near the
// view are drawn, and a file's diff, or its next page, is asked for when
// its row is drawn. A selection of its lines can be sent to the composer.

import { useEffect, useMemo } from "react";

import {
  changeRows,
  type ChangeRow,
  type FileState,
} from "../../data/workspace/changes";
import { languageOf } from "../../data/workspace/syntax-languages";
import type { FileChange, RelPath, SessionId } from "../../wire/proto";
import { useWindow } from "../session/use-window";
import { WindowRow } from "../session/window-row";
import { ChangeRowView, type FileActions } from "./change-row";
import { SelectionAsk } from "./selection-ask";
import type { ReadRows } from "./use-changes";
import { useSelectionAsk } from "./use-selection";

/** The file a row's key names (`changes.ts`'s `rowPrefix`). */
function pathOf(row: ChangeRow): RelPath {
  const at = row.key.indexOf("\u0000");
  return at < 0 ? row.key : row.key.slice(0, at);
}

/** A fence's language for a path's lines. */
function fenceOf(path: string): string {
  const name = languageOf(path).name;
  return name === "text" ? "" : name;
}

export function ChangesList({
  session,
  files,
  states,
  want,
  closed,
  actions,
}: {
  session: SessionId;
  files: readonly FileChange[];
  states: ReadonlyMap<RelPath, FileState>;
  want: (path: RelPath, after: ReadRows | null) => void;
  /** The files folded to their header. */
  closed: ReadonlySet<RelPath>;
  actions: FileActions;
}) {
  const rows = useMemo(
    () => changeRows(files, closed, states),
    [files, closed, states],
  );
  const windowed = useWindow(rows, null, { follow: false });
  const { scroller, span } = windowed;
  const drawn = useMemo(
    () => rows.slice(span.start, span.end),
    [rows, span.start, span.end],
  );
  // The diffs and pages the drawn rows stand for.
  useEffect(() => {
    for (const row of drawn) {
      if (row.kind !== "note") {
        continue;
      }
      if (row.note.kind === "loading") {
        want(row.path, null);
      } else if (row.note.kind === "more") {
        const state = states.get(row.path);
        if (state?.kind === "read" && state.body.kind === "rows") {
          want(row.path, state.body);
        }
      }
    }
  }, [drawn, states, want]);
  const selection = useSelectionAsk(scroller, session, fenceOf);
  return (
    <div className="changes-scroll" ref={scroller}>
      <section className="changes-list" aria-label="Changed files">
        <div style={{ height: span.before }} />
        {drawn.map((row, index) => (
          <WindowRow
            key={row.key}
            id={row.key}
            last={span.start + index === rows.length - 1}
            rows={windowed.rows}
          >
            <ChangeRowView row={row} path={pathOf(row)} actions={actions} />
          </WindowRow>
        ))}
        <div style={{ height: span.after }} />
      </section>
      {selection.ask === null ? null : (
        <SelectionAsk
          ask={selection.ask}
          onSend={selection.send}
          onDismiss={selection.dismiss}
        />
      )}
    </div>
  );
}
