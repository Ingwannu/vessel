// The Explorer's file actions (Paseo's FileActionsContextMenuContent,
// file-actions-menu.tsx): the rows a tree entry offers, in Paseo's order
// and groups, with a hairline where the group changes. The screen says
// which it can do (`can`); the entry's kind and whether a folder is open
// decide the rest, as Paseo's props do.
//
// Paseo's Open in editor, Reveal in file manager (desktop only) and Open
// to the side (a second pane) are not here: the page has none of them.

import type { IconName } from "../../ui/kit";

export type FileActionKey =
  | "new-file"
  | "new-folder"
  | "collapse-folder"
  | "copy-path"
  | "copy-relative-path"
  | "download"
  | "add-to-chat"
  | "rename"
  | "duplicate"
  | "delete";

type Group = "create" | "open" | "reference" | "manage" | "destructive";

/** Which rows apply to an entry: a file's, a folder's, an open folder's. */
type Applies = "any" | "file" | "open-folder";

interface Spec {
  key: FileActionKey;
  group: Group;
  label: string;
  icon: IconName;
  applies: Applies;
}

/** Paseo's order (file-actions-menu.tsx 112-237) and English words
 * (`workspace.fileActions`). */
const SPECS: readonly Spec[] = [
  {
    key: "new-file",
    group: "create",
    label: "New file",
    icon: "file",
    applies: "any",
  },
  {
    key: "new-folder",
    group: "create",
    label: "New folder",
    icon: "folder-plus",
    applies: "any",
  },
  {
    key: "collapse-folder",
    group: "open",
    label: "Collapse folder",
    icon: "fold",
    applies: "open-folder",
  },
  {
    key: "copy-path",
    group: "reference",
    label: "Copy path",
    icon: "copy",
    applies: "any",
  },
  {
    key: "copy-relative-path",
    group: "reference",
    label: "Copy relative path",
    icon: "copy",
    applies: "any",
  },
  {
    key: "download",
    group: "reference",
    label: "Download",
    icon: "download",
    applies: "file",
  },
  {
    key: "add-to-chat",
    group: "reference",
    label: "Add to chat",
    icon: "message",
    applies: "file",
  },
  {
    key: "rename",
    group: "manage",
    label: "Rename",
    icon: "edit",
    applies: "any",
  },
  {
    key: "duplicate",
    group: "manage",
    label: "Duplicate",
    icon: "copy",
    applies: "any",
  },
  {
    key: "delete",
    group: "destructive",
    label: "Delete",
    icon: "trash",
    applies: "any",
  },
];

export interface FileAction {
  key: FileActionKey;
  label: string;
  icon: IconName;
  /** Red, as Paseo's destructive rows. */
  destructive: boolean;
  /** A hairline above it: its group differs from the row before. */
  separatorBefore: boolean;
}

export interface FileActionEntry {
  kind: "file" | "dir";
  /** A folder open in the tree (its Collapse). */
  open: boolean;
}

function applies(spec: Spec, entry: FileActionEntry): boolean {
  switch (spec.applies) {
    case "any":
      return true;
    case "file":
      return entry.kind === "file";
    case "open-folder":
      return entry.kind === "dir" && entry.open;
  }
}

/** The rows `entry` offers, of those the screen `can` do. */
export function fileActions(
  entry: FileActionEntry,
  can: ReadonlySet<FileActionKey>,
): FileAction[] {
  const chosen = SPECS.filter(
    (spec) => can.has(spec.key) && applies(spec, entry),
  );
  return chosen.map((spec, index) => ({
    key: spec.key,
    label: spec.label,
    icon: spec.icon,
    destructive: spec.group === "destructive",
    separatorBefore: index > 0 && chosen[index - 1]?.group !== spec.group,
  }));
}
