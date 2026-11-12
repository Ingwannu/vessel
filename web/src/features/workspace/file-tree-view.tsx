// The Files tab's tree (agent-orchestrator's file tree): 28px rows, 6px
// in, a folder's open or closed mark or a file's, the name at 13px, a 4%
// wash on hover and 7% on the open file; each level 16px further in. A
// folder is listed by the node when it is first opened; the rows are
// windowed as the timeline's are.

import { useCallback, useEffect, useMemo, useState } from "react";

import { useFleet } from "../../context";
import {
  type Listing,
  treeRows,
  type TreeRow,
} from "../../data/workspace/file-tree";
import { listDir } from "../../data/workspace/files";
import { Icon, Spinner } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, WorkspaceRoot } from "../../wire/proto";
import { useWindow } from "../session/use-window";
import { WindowRow } from "../session/window-row";

function TreeRowView({
  row,
  onOpen,
}: {
  row: TreeRow;
  onOpen: (path: RelPath, dir: boolean) => void;
}) {
  const indent = { paddingLeft: 6 + row.depth * 16 };
  switch (row.kind) {
    case "entry": {
      const dir = row.entry.kind === "dir";
      return (
        <button
          type="button"
          className={
            row.entry.ignored === true ? "tree-row ignored" : "tree-row"
          }
          style={indent}
          aria-expanded={dir ? row.open : undefined}
          onClick={() => {
            onOpen(row.path, dir);
          }}
        >
          <Icon
            name={dir ? (row.open ? "folder-open" : "folder") : "file"}
            size={14}
          />
          <span className="tree-name">{row.entry.name}</span>
          {row.entry.kind === "symlink" ? (
            <span className="tree-note">link</span>
          ) : null}
        </button>
      );
    }
    case "loading":
      return (
        <p className="tree-note-row" style={indent}>
          <Spinner size={12} /> Loading…
        </p>
      );
    case "failed":
      return (
        <p className="tree-note-row failed" style={indent}>
          {row.message}
        </p>
      );
    case "empty":
      return (
        <p className="tree-note-row" style={indent}>
          This folder is empty.
        </p>
      );
    case "more":
      return (
        <p className="tree-note-row" style={indent}>
          Only the first 5,000 entries are listed; find a file by name above.
        </p>
      );
  }
}

export function FileTreeView({
  node,
  root,
  ignored,
  open,
  onOpenChange,
  onFile,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  /** Whether files git ignores are listed too. */
  ignored: boolean;
  /** The folders open, kept by the tab across a file opened. */
  open: ReadonlySet<RelPath>;
  onOpenChange: (open: ReadonlySet<RelPath>) => void;
  onFile: (path: RelPath) => void;
}) {
  const fleet = useFleet();
  const [listings, setListings] = useState<ReadonlyMap<RelPath, Listing>>(
    () => new Map(),
  );
  // The folders open when this tree was drawn (a refresh draws a new
  // one): listed with the root, so they come back as they are on disk.
  const [listedFirst] = useState(open);
  // A folder's listing, asked of the node; until it answers, a folder
  // with no listing draws as loading (`treeRows`).
  const fetchListing = useCallback(
    (dir: RelPath) => {
      void listDir(fleet, node, root, dir, ignored).then((outcome) => {
        setListings((known) =>
          new Map(known).set(
            dir,
            outcome.ok
              ? { kind: "listed", listing: outcome.value }
              : { kind: "failed", message: outcome.problem.message },
          ),
        );
      });
    },
    [fleet, node, root, ignored],
  );
  useEffect(() => {
    fetchListing("");
    for (const dir of listedFirst) {
      fetchListing(dir);
    }
  }, [fetchListing, listedFirst]);
  const rows = useMemo(() => treeRows(listings, open), [listings, open]);
  const windowed = useWindow(rows, null, { follow: false });
  const { scroller, span } = windowed;
  const onOpen = (path: RelPath, dir: boolean) => {
    if (!dir) {
      onFile(path);
      return;
    }
    const next = new Set(open);
    if (!next.delete(path)) {
      next.add(path);
      if (!listings.has(path)) {
        fetchListing(path);
      }
    }
    onOpenChange(next);
  };
  return (
    <div className="file-scroll" ref={scroller}>
      <section aria-label="Files" className="tree">
        <div style={{ height: span.before }} />
        {rows.slice(span.start, span.end).map((row, offset) => (
          <WindowRow
            key={row.key}
            id={row.key}
            last={span.start + offset === rows.length - 1}
            rows={windowed.rows}
          >
            <TreeRowView row={row} onOpen={onOpen} />
          </WindowRow>
        ))}
        <div style={{ height: span.after }} />
      </section>
    </div>
  );
}
