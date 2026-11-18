// The Files tab's tree (agent-orchestrator's file tree): 28px rows, 6px
// in, a folder's open or closed mark or a file's, the name at 13px, a 4%
// wash on hover and 7% on the open file; each level 16px further in. A
// folder is listed by the node when it is first opened; the rows are
// windowed as the timeline's are. A row's right click (or a held touch)
// opens Paseo's file actions menu (`explorer-menu.tsx`).

import {
  type Dispatch,
  type SetStateAction,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";

import { useFleet } from "../../context";
import {
  type Listing,
  treeRows,
  type TreeRow,
  withEdit,
} from "../../data/workspace/file-tree";
import { listDir } from "../../data/workspace/files";
import { Icon, Spinner } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionId, WorkspaceRoot } from "../../wire/proto";
import { useWindow } from "../session/use-window";
import { WindowRow } from "../session/window-row";
import { DeleteConfirm, EntryMenu, NameRow, SpaceMenu } from "./explorer-menu";
import {
  type ExplorerActions,
  useExplorerActions,
} from "./use-explorer-actions";

function TreeRowView({
  row,
  onOpen,
  actions,
}: {
  row: TreeRow;
  onOpen: (path: RelPath, dir: boolean) => void;
  actions: ExplorerActions;
}) {
  const indent = { paddingLeft: 6 + row.depth * 16 };
  switch (row.kind) {
    case "entry": {
      const dir = row.entry.kind === "dir";
      const button = (
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
      return (
        <EntryMenu
          trigger={button}
          // A link acts as the file it leads to (the node reads through it).
          entry={{ kind: dir ? "dir" : "file", open: row.open }}
          target={{
            path: row.path,
            name: row.entry.name,
            kind: dir ? "dir" : "file",
          }}
          actions={actions}
        />
      );
    }
    case "edit":
      return (
        <NameRow
          entry={row.entry}
          initial={row.initial}
          depth={row.depth}
          actions={actions}
        />
      );
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
  folder,
  session,
  ignored,
  open,
  onOpenChange,
  onFile,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  /** The root's folder on the node (Copy path), when known. */
  folder: string | null;
  /** The session whose composer Add to chat writes in. */
  session: SessionId;
  /** Whether files git ignores are listed too. */
  ignored: boolean;
  /** The folders open, kept by the tab across a file opened. */
  open: ReadonlySet<RelPath>;
  /** The tab's setter: a set, or a change of the set as it is when it
   * runs (a rename or a delete answers after the tree may have moved). */
  onOpenChange: Dispatch<SetStateAction<ReadonlySet<RelPath>>>;
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
  const actions = useExplorerActions(
    { node, root, folder, session },
    {
      open,
      onOpenChange,
      refresh: fetchListing,
      expand: (dir) => {
        if (open.has(dir)) return;
        if (!listings.has(dir)) fetchListing(dir);
        onOpenChange(new Set(open).add(dir));
      },
      onFile,
    },
  );
  const rows = useMemo(
    () => withEdit(treeRows(listings, open), actions.edit),
    [listings, open, actions.edit],
  );
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
            <TreeRowView row={row} onOpen={onOpen} actions={actions} />
          </WindowRow>
        ))}
        <div style={{ height: span.after }} />
      </section>
      <SpaceMenu actions={actions} />
      <DeleteConfirm actions={actions} />
    </div>
  );
}
