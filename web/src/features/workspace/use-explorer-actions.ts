// What the Explorer's menu does on a row (Paseo's file-explorer-pane.tsx
// handlers behind file-actions-menu.tsx): which rows this screen can
// offer, and each one's work.
//
// [Decision Log]
// - Purpose: the page Explorer's file action menu (TRACKER: the whole
//   menu was missing); E1 holds the rows that change no file.
// - Constraint: the page has no editor, file manager or second pane
//   (Open in editor, Reveal, Open to the side stay out); `file.read_bytes`
//   is the file panels protocol's (27), so an older node offers no
//   Download; Copy path needs the root's folder, which a worktree session
//   knows as its cwd (the node starts it there).
// - Choice: Paseo's handlers one for one: copies say nothing (Paseo's
//   explorer has no copied toast), a refused clipboard or a failed read
//   says so in an error toast; Add to chat names the file in the
//   session's draft as the composer's `@` does.
// - Cost: Add to chat writes `@path` where Paseo adds an attachment pill;
//   the page's composer has no file pills, and the agent reads `@path`.

import { useMemo } from "react";

import { useFleet, useFleetState } from "../../context";
import { withMention } from "../../data/suggest";
import { absolutePath } from "../../data/workspace/file-tree";
import { hasFilePanels } from "../../data/workspace/files";
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

/** The row a menu acts on. */
export interface ExplorerTarget {
  path: RelPath;
  name: string;
}

export interface ExplorerActions {
  can: ReadonlySet<FileActionKey>;
  act: (key: FileActionKey, target: ExplorerTarget) => void;
}

function copy(text: string): void {
  void navigator.clipboard.writeText(text).catch(() => {
    toast("The clipboard refused the text", "error");
  });
}

export function useExplorerActions(
  place: ExplorerPlace,
  open: ReadonlySet<RelPath>,
  onOpenChange: (open: ReadonlySet<RelPath>) => void,
): ExplorerActions {
  const fleet = useFleet();
  const { node, root, folder, session } = place;
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  const can = useMemo(() => {
    const keys: FileActionKey[] = [
      "collapse-folder",
      "copy-relative-path",
      "add-to-chat",
    ];
    if (folder !== null) keys.push("copy-path");
    if (hasFilePanels(protocol)) keys.push("download");
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
      case "new-folder":
      case "rename":
      case "duplicate":
      case "delete":
        // Not offered yet (`can`): the second packet's.
        return;
    }
  };
  return { can, act };
}
