// Owns one file's editor and save guard. The tab keeps it mounted so
// switching away preserves the original version and any unsaved text.

import "./files.css";

import { useEffect, useState } from "react";

import {
  filePanelOf,
  hasPreview,
  imageType,
  previewFits,
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
  active,
  onDirtyChange,
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
  active: boolean;
  onDirtyChange: (dirty: boolean) => void;
}) {
  const file = useFile(node, root, path, active);
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
  // Past Paseo's display budget the Preview says so instead of reading
  // the whole file; Source still reads it a page at a time.
  const tooLargeToPreview =
    text.kind === "text" && !previewFits(text.version.size);
  const editing = draft !== null;
  const dirty =
    editing && (text.kind !== "text" || draft !== text.lines.join("\n"));
  useEffect(() => {
    onDirtyChange(dirty || saving);
  }, [dirty, saving, onDirtyChange]);
  const tooBig = text.kind === "text" && text.version.size > WRITE_LIMIT;
  // The disk moved past what is shown (the file's watch): the view follows
  // it; an open edit keeps its text under the alert, and its save then
  // meets the change as the conflict below. A file gone keeps its copy.
  const { disk, refresh } = file;
  useEffect(() => {
    if (active && disk === "changed" && draft === null) {
      refresh();
    }
  }, [active, disk, draft, refresh]);

  // A Preview draws the whole file: the pages not read yet are read.
  const partial = text.kind === "text" && text.next !== null;
  const { whole } = file;
  useEffect(() => {
    if (active && previewing && partial && !tooLargeToPreview) {
      void whole();
    }
  }, [active, previewing, partial, tooLargeToPreview, whole]);

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
      ) : previewing && tooLargeToPreview ? (
        <EmptyState title="This file is too large to display" icon="file">
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
