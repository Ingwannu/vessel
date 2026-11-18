// What the Explorer's menu does on a row (Paseo's file-explorer-pane.tsx
// handlers behind file-actions-menu.tsx): which rows this screen can
// offer, and each one's work.
//
// [Decision Log]
// - Purpose: the page Explorer's file action menu (TRACKER: the whole
//   menu was missing): E1 the rows that change no file, E2 New file, New
//   folder, Rename, Duplicate and Delete.
// - Constraint: the page has no editor, file manager or second pane
//   (Open in editor, Reveal, Open to the side stay out); `file.read_bytes`
//   is the file panels protocol's (27), so an older node offers no
//   Download; Copy path needs the root's folder, which a worktree session
//   knows as its cwd (the node starts it there).
// - Choice: Paseo's handlers one for one: copies say nothing (Paseo's
//   explorer has no copied toast), a refused clipboard or a failed read
//   says so in an error toast; Add to chat names the file in the
//   session's draft as the composer's `@` does.
// - E2, as Paseo's handlers: New file and New folder open the folder and
//   type the name in a row at its top, Rename types it in the entry's
//   place (Enter or leaving the field makes it, Escape or an empty name
//   drops it); a made file opens; a new file is written only where none
//   is (`expect: absent`), a rename never lands on an existing path (the
//   node's rule); Delete asks first in Paseo's words, a folder with what
//   is inside it; Duplicate is the node's `file.duplicate` (protocol 27).
//   Each lists the folders it touched again, and the open folders follow
//   a rename or a delete (Paseo's replaceExplorerPathPrefix).
// - Cost: Add to chat writes `@path` where Paseo adds an attachment pill;
//   the page's composer has no file pills, and the agent reads `@path`.

import { type Dispatch, type SetStateAction, useMemo, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { withMention } from "../../data/suggest";
import {
  absolutePath,
  childPath,
  isWithin,
  nameProblem,
  parentOf,
  renamedOpen,
  type TreeEdit,
  withoutWithin,
} from "../../data/workspace/file-tree";
import {
  createDir,
  deletePath,
  duplicateEntry,
  hasFilePanels,
  renamePath,
  writeFile,
} from "../../data/workspace/files";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionId, WorkspaceRoot } from "../../wire/proto";
import { downloadFile } from "./download";
import type { FileActionKey } from "./file-actions";

/** Where the Explorer is: its root, the root's folder on the node (for
 * Copy path; `null` when unknown) and the session whose composer Add to
 * chat writes in. */
export interface ExplorerPlace {
  node: NodeKey;
  root: WorkspaceRoot;
  folder: string | null;
  session: SessionId;
}

/** The row a menu acts on (the tree's empty space: the root, `""`). */
export interface ExplorerTarget {
  path: RelPath;
  name: string;
  kind: "file" | "dir";
}

/** What the actions do to the tree drawn around them. */
export interface ExplorerTree {
  open: ReadonlySet<RelPath>;
  /** A set, or a change of the set as it is when it runs. */
  onOpenChange: Dispatch<SetStateAction<ReadonlySet<RelPath>>>;
  /** List folder `dir` again: something in it changed. */
  refresh: (dir: RelPath) => void;
  /** Open folder `dir`, listing it if it has not been. */
  expand: (dir: RelPath) => void;
  onFile: (path: RelPath) => void;
}

export interface ExplorerActions {
  can: ReadonlySet<FileActionKey>;
  act: (key: FileActionKey, target: ExplorerTarget) => void;
  /** The name being typed in the tree, and its end. */
  edit: TreeEdit | null;
  commit: (name: string) => void;
  cancel: () => void;
  /** The entry Delete asks about, and the answer. */
  deleting: ExplorerTarget | null;
  confirmDelete: () => void;
  cancelDelete: () => void;
}

function copy(text: string): void {
  void navigator.clipboard.writeText(text).catch(() => {
    toast("The clipboard refused the text", "error");
  });
}

/** Say a refusal in the node's words (Paseo's toast.error). */
function refused(outcome: { ok: false; problem: { message: string } }): void {
  toast(outcome.problem.message, "error");
}

export function useExplorerActions(
  place: ExplorerPlace,
  tree: ExplorerTree,
): ExplorerActions {
  const fleet = useFleet();
  const { node, root, folder, session } = place;
  const { open, onOpenChange, refresh, expand, onFile } = tree;
  const [edit, setEdit] = useState<TreeEdit | null>(null);
  const [deleting, setDeleting] = useState<ExplorerTarget | null>(null);
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const can = useMemo(() => {
    const keys: FileActionKey[] = [
      "new-file",
      "new-folder",
      "collapse-folder",
      "copy-relative-path",
      "add-to-chat",
      "rename",
      "delete",
    ];
    if (folder !== null) keys.push("copy-path");
    if (hasFilePanels(protocol)) keys.push("download", "duplicate");
    return new Set(keys);
  }, [folder, protocol]);
  const act = (key: FileActionKey, target: ExplorerTarget) => {
    switch (key) {
      case "collapse-folder": {
        const next = new Set(open);
        next.delete(target.path);
        onOpenChange(next);
        return;
      }
      case "copy-path":
        if (folder !== null) copy(absolutePath(folder, target.path));
        return;
      case "copy-relative-path":
        copy(target.path);
        return;
      case "download":
        void downloadFile(fleet, node, root, target.path, target.name).then(
          (saved) => {
            if (!saved.ok) toast(saved.problem.message, "error");
          },
        );
        return;
      case "add-to-chat": {
        const draft = { kind: "session" as const, session };
        fleet.drafts.type(
          draft,
          withMention(fleet.drafts.text(draft), target.path),
        );
        return;
      }
      case "new-file":
      case "new-folder": {
        // Paseo's: in the folder itself, or beside the file.
        const parent =
          target.kind === "dir" ? target.path : parentOf(target.path);
        if (parent !== "") expand(parent);
        setEdit({
          kind: "create",
          parent,
          entry: key === "new-file" ? "file" : "dir",
        });
        return;
      }
      case "rename":
        setEdit({
          kind: "rename",
          path: target.path,
          entry: target.kind,
          name: target.name,
        });
        return;
      case "duplicate":
        void duplicateEntry(fleet, node, root, target.path).then((made) => {
          if (made.ok) refresh(parentOf(made.value.path));
          else refused(made);
        });
        return;
      case "delete":
        setDeleting(target);
        return;
    }
  };
  const create = (
    made: Extract<TreeEdit, { kind: "create" }>,
    name: string,
  ) => {
    const path = childPath(made.parent, name);
    if (made.entry === "dir") {
      void createDir(fleet, node, root, path).then((outcome) => {
        if (outcome.ok) refresh(made.parent);
        else refused(outcome);
      });
      return;
    }
    // Only where no file is: a name already taken is refused, not emptied.
    void writeFile(fleet, node, root, path, "", { kind: "absent" }).then(
      (outcome) => {
        if (!outcome.ok) {
          refused(outcome);
          return;
        }
        refresh(made.parent);
        onFile(path);
      },
    );
  };
  const rename = (
    renamed: Extract<TreeEdit, { kind: "rename" }>,
    name: string,
  ) => {
    if (name === renamed.name) return;
    const parent = parentOf(renamed.path);
    const to = childPath(parent, name);
    void renamePath(fleet, node, root, renamed.path, to).then((outcome) => {
      if (!outcome.ok) {
        refused(outcome);
        return;
      }
      refresh(parent);
      if (renamed.entry === "dir") {
        // The open set as it is now: the answer may come after the tree
        // moved on (the reviewer's note on E2).
        onOpenChange((now) => renamedOpen(now, renamed.path, to));
        for (const dir of renamedOpen(open, renamed.path, to)) {
          if (isWithin(dir, to)) refresh(dir);
        }
      }
    });
  };
  const commit = (name: string) => {
    const typed = edit;
    setEdit(null);
    if (typed === null) return;
    const problem = nameProblem(name);
    if (problem !== null) {
      toast(problem, "error");
      return;
    }
    const trimmed = name.trim();
    if (typed.kind === "create") create(typed, trimmed);
    else rename(typed, trimmed);
  };
  const confirmDelete = () => {
    const target = deleting;
    setDeleting(null);
    if (target === null) return;
    const recursive = target.kind === "dir";
    void deletePath(fleet, node, root, target.path, recursive).then(
      (outcome) => {
        if (!outcome.ok) {
          refused(outcome);
          return;
        }
        refresh(parentOf(target.path));
        onOpenChange((now) => withoutWithin(now, target.path));
      },
    );
  };
  return {
    can,
    act,
    edit,
    commit,
    cancel: () => {
      setEdit(null);
    },
    deleting,
    confirmDelete,
    cancelDelete: () => {
      setDeleting(null);
    },
  };
}
