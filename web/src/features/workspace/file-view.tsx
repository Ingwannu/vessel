// One file in the Files tab (agent-orchestrator's file pane): a 36px head
// with the way back, the file's path (its folders muted, its name in the
// ink) and Edit; the text in its colours; or the editor, with Cancel and
// Save (⌘/Ctrl+S) and a dot while there are unsaved edits. A save that
// finds the file changed on disk keeps the edits and shows both sides
// (GOAT's banner: reload from disk, or overwrite). The file's watch
// (ADR 0105) follows the disk with no edit open; with one open, or when
// the file is gone, Paseo's conflict alert says so under the head.
//
// Paseo's built-in panels (F-F38, F-F39, F-F40;
// docs/plans/wp-file-viewers.md) pick how the file shows by its name:
// Markdown and HTML have a Preview beside their Source, opened on Preview
// unless the file was opened at a line (each later line asked for turns
// to Source again); an image is drawn, zoomable, from its bytes on a node
// that reads them (`file.read_bytes`); anything else is its Source. The
// Preview draws the edit while one is open, as Paseo's draws its buffer.

import "./files.css";

import { useEffect, useState } from "react";

import {
  filePanelOf,
  hasPreview,
  imageType,
} from "../../data/workspace/file-panels";
import { nodeError } from "../../data/workspace/root";
import type { Saved } from "../../data/workspace/save";
import { sizeWord } from "../../data/workspace/size";
import { languageOf } from "../../data/workspace/syntax-languages";
import { Banner, EmptyState, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  FileVersion,
  RelPath,
  SessionId,
  WorkspaceRoot,
} from "../../wire/proto";
import {
  type DiskChange,
  FileChangeAlert,
  ReloadDialog,
} from "./file-change-alert";
import { FileConflict } from "./file-conflict";
import { FileEditor } from "./file-editor";
import { FileHead, useSide } from "./file-head";
import { FileImage } from "./file-image";
import { FileLines } from "./file-lines";
import { FilePreview } from "./file-preview";
import { useFile, versionKey, WRITE_LIMIT } from "./use-file";

export function FileView({
  node,
  root,
  folder,
  session,
  path,
  goTo,
  filePanels,
  onBack,
  onOpenFile,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  /** The root's folder on the node, when known. */
  folder: string | null;
  session: SessionId;
  path: RelPath;
  goTo: { line: number; seq: number } | null;
  /** The node reads files' bytes (`file.read_bytes`): images are drawn. */
  filePanels: boolean;
  onBack: () => void;
  /** Open another file of the workspace (a Markdown link). */
  onOpenFile: (path: RelPath) => void;
}) {
  const file = useFile(node, root, path);
  const [draft, setDraft] = useState<string | null>(null);
  // The version the edit started from: the save's guard, whatever is
  // read after it.
  const [base, setBase] = useState<FileVersion | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<Saved | null>(null);
  const [confirmReload, setConfirmReload] = useState(false);
  const language = languageOf(path);
  const text = file.text;
  const panel = filePanelOf(path);
  const preview = hasPreview(panel) ? panel : null;
  const image = panel === "image" && filePanels ? imageType(path) : null;
  const [side, setSide] = useSide(goTo);
  const previewing = preview !== null && side === "preview";
  const editing = draft !== null;
  const dirty =
    editing && (text.kind !== "text" || draft !== text.lines.join("\n"));
  const tooBig = text.kind === "text" && text.version.size > WRITE_LIMIT;
  // The disk moved past what is shown (the file's watch): the view follows
  // it; an open edit keeps its text under the alert, and its save then
  // meets the change as the conflict below. A file gone keeps its copy.
  const { disk, refresh } = file;
  useEffect(() => {
    if (disk === "changed" && draft === null) {
      refresh();
    }
  }, [disk, draft, refresh]);

  // A Preview draws the whole file: the pages not read yet are read.
  const partial = text.kind === "text" && text.next !== null;
  const { whole } = file;
  useEffect(() => {
    if (previewing && partial) {
      void whole();
    }
  }, [previewing, partial, whole]);

  const save = (force: boolean) => {
    if (draft === null || saving) {
      return;
    }
    setSaving(true);
    void file.save(draft, force ? null : base).then((answer) => {
      setSaving(false);
      setSaved(answer);
      if (answer.kind === "saved") {
        setDraft(null);
      }
    });
  };
  const reloadFromDisk = () => {
    setConfirmReload(false);
    setDraft(null);
    setSaved(null);
    file.reload();
  };
  const shownCopy = editing || text.kind === "text" || text.kind === "binary";
  const change: DiskChange | null =
    saved?.kind === "conflict" && editing
      ? null
      : disk === "deleted" && shownCopy
        ? { kind: "deleted" }
        : disk === "changed" && editing
          ? {
              kind: "changed",
              canOverwrite: dirty,
              busy: saving,
              onOverwrite: () => {
                save(true);
              },
              onReload: () => {
                // Paseo asks only when the edit would be thrown away.
                if (dirty) {
                  setConfirmReload(true);
                } else {
                  reloadFromDisk();
                }
              },
            }
          : null;
  const edit = () => {
    setSide("source");
    void file.whole().then((whole) => {
      if (whole !== null) {
        setSaved(null);
        setBase(whole.version);
        setDraft(whole.text);
      }
    });
  };

  return (
    <div className="file-view">
      <FileHead
        path={path}
        dirty={dirty}
        editing={editing}
        saving={saving}
        edit={
          text.kind !== "text" || image !== null
            ? undefined
            : tooBig
              ? null
              : edit
        }
        side={preview !== null && text.kind === "text" ? side : null}
        onSide={setSide}
        onBack={onBack}
        onCancel={() => {
          setDraft(null);
          setSaved(null);
        }}
        onSave={() => {
          save(false);
        }}
      />
      {change === null ? null : <FileChangeAlert change={change} />}
      {text.kind === "loading" ? (
        <div className="changes-pad">
          <Skeleton />
        </div>
      ) : text.kind === "failed" ? (
        <EmptyState title="This file cannot be shown" icon="file">
          {nodeError(text.problem) === "not_found"
            ? "It is gone, or it is outside this folder."
            : text.problem.message}
        </EmptyState>
      ) : image !== null ? (
        <FileImage
          node={node}
          root={root}
          path={path}
          name={path.split("/").pop() ?? path}
          type={image}
          version={versionKey(text.version)}
        />
      ) : text.kind === "binary" ? (
        <EmptyState title="Binary file preview is not available." icon="file">
          {sizeWord(text.version.size)}
        </EmptyState>
      ) : previewing ? (
        <FilePreview
          panel={preview}
          node={node}
          root={root}
          folder={folder}
          path={path}
          text={draft ?? text.lines.join("\n")}
          version={versionKey(text.version)}
          onOpenFile={onOpenFile}
        />
      ) : draft !== null ? (
        <FileEditor
          path={path}
          draft={draft}
          saving={saving}
          onChange={setDraft}
          onSave={() => {
            save(false);
          }}
        />
      ) : (
        <FileLines
          session={session}
          path={path}
          lines={text.lines}
          language={language}
          hasMore={text.next !== null}
          onMore={file.more}
          goTo={goTo}
        />
      )}
      {saved?.kind === "conflict" && draft !== null && base !== null ? (
        <FileConflict
          mine={saved.mine}
          read={base}
          disk={saved.disk}
          diskText={saved.diskText}
          busy={saving}
          onReload={() => {
            setDraft(null);
            setSaved(null);
            file.reload();
          }}
          onOverwrite={() => {
            save(true);
          }}
        />
      ) : saved?.kind === "failed" ? (
        <div className="file-strip">
          <Banner tone="error">{saved.message}</Banner>
        </div>
      ) : null}
      {confirmReload ? (
        <ReloadDialog
          onCancel={() => {
            setConfirmReload(false);
          }}
          onReload={reloadFromDisk}
        />
      ) : null}
    </div>
  );
}
