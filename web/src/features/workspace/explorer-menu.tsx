// The Explorer's menus and the parts its actions draw (Paseo's
// file-explorer-pane.tsx): the menu a tree row opens on a right click or a
// held touch (344-396 around file-actions-menu.tsx), the empty space's
// (1351: New file, New folder at the root), the row a name is typed in
// (EntryNameInputRow, 164-230), and Delete's question (its confirmDialog,
// 805-818). The rows come in Paseo's order and groups (`file-actions.ts`),
// a hairline between groups, Delete red.

import {
  Fragment,
  type ReactElement,
  useEffect,
  useRef,
  useState,
} from "react";

import {
  Button,
  ContextMenu,
  Dialog,
  Icon,
  MenuItem,
  MenuSeparator,
  TextField,
} from "../../ui/kit";
import {
  type FileAction,
  type FileActionEntry,
  fileActions,
  spaceActions,
} from "./file-actions";
import type { ExplorerActions, ExplorerTarget } from "./use-explorer-actions";

function Rows({
  rows,
  target,
  actions,
}: {
  rows: readonly FileAction[];
  target: ExplorerTarget;
  actions: ExplorerActions;
}) {
  return rows.map((row) => (
    <Fragment key={row.key}>
      {row.separatorBefore ? <MenuSeparator /> : null}
      <MenuItem
        icon={row.icon}
        tone={row.destructive ? "danger" : "default"}
        onSelect={() => {
          actions.act(row.key, target);
        }}
      >
        {row.label}
      </MenuItem>
    </Fragment>
  ));
}

export function EntryMenu({
  trigger,
  entry,
  target,
  actions,
}: {
  trigger: ReactElement;
  entry: FileActionEntry;
  target: ExplorerTarget;
  actions: ExplorerActions;
}) {
  const rows = fileActions(entry, actions.can);
  // No row to offer: the browser's own menu stays, as on any page.
  if (rows.length === 0) return trigger;
  return (
    <ContextMenu trigger={trigger}>
      <Rows rows={rows} target={target} actions={actions} />
    </ContextMenu>
  );
}

const ROOT: ExplorerTarget = { path: "", name: "", kind: "dir" };

/** The tree's empty space below its rows: New file and New folder at the
 * root. */
export function SpaceMenu({ actions }: { actions: ExplorerActions }) {
  const rows = spaceActions(actions.can);
  const space = <div className="tree-space" />;
  if (rows.length === 0) return space;
  return (
    <ContextMenu trigger={space}>
      <Rows rows={rows} target={ROOT} actions={actions} />
    </ContextMenu>
  );
}

/** A name typed in the tree: Enter or leaving the field makes it, Escape
 * or an empty name drops it (Paseo's EntryNameInputRow). */
export function NameRow({
  entry,
  initial,
  depth,
  actions,
}: {
  entry: "file" | "dir";
  initial: string;
  depth: number;
  actions: ExplorerActions;
}) {
  const [name, setName] = useState(initial);
  // Enter, then the blur it causes, must not make it twice.
  const settled = useRef(false);
  const field = useRef<HTMLInputElement>(null);
  // Paseo's `selectTextOnFocus` for a rename: the whole name, to retype.
  useEffect(() => {
    if (initial !== "") field.current?.select();
  }, [initial]);
  const commit = () => {
    if (settled.current) return;
    settled.current = true;
    if (name.trim() === "") actions.cancel();
    else actions.commit(name);
  };
  const placeholder = entry === "dir" ? "Folder name" : "File name";
  return (
    <div className="tree-row editing" style={{ paddingLeft: 6 + depth * 16 }}>
      <Icon name={entry === "dir" ? "folder" : "file"} size={14} />
      <span className="tree-name-field">
        <TextField
          size="sm"
          value={name}
          onChange={setName}
          placeholder={placeholder}
          aria-label={placeholder}
          spellCheck={false}
          // Paseo's row takes focus as it appears (`autoFocus`).
          autoFocus
          inputRef={field}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
            } else if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              settled.current = true;
              actions.cancel();
            }
          }}
          onBlur={commit}
        />
      </span>
    </div>
  );
}

/** Delete's question, in Paseo's words (workspace.fileActions
 * .confirmDelete). */
export function DeleteConfirm({ actions }: { actions: ExplorerActions }) {
  const target = actions.deleting;
  if (target === null) return null;
  return (
    <Dialog
      title={target.kind === "dir" ? "Delete folder?" : "Delete file?"}
      onClose={actions.cancelDelete}
      footer={
        <>
          <Button variant="ghost" onClick={actions.cancelDelete}>
            Cancel
          </Button>
          <Button variant="danger-solid" onClick={actions.confirmDelete}>
            Delete
          </Button>
        </>
      }
    >
      <p className="tree-delete-words">
        {`"${target.name}" will be permanently deleted.`}
      </p>
    </Dialog>
  );
}
