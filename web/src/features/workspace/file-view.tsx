// One file in the Files tab (agent-orchestrator's file pane): a 36px head
// with the way back, the file's path (its folders muted, its name in the
// ink) and Edit; the text in its colours; or the editor, with Cancel and
// Save (⌘/Ctrl+S) and a dot while there are unsaved edits. A save that
// finds the file changed on disk keeps the edits and shows both sides
// (GOAT's banner: reload from disk, or overwrite).

import "./files.css";

import { useEffect, useState } from "react";

import { splitPath } from "../../data/workspace/changes";
import { nodeError } from "../../data/workspace/root";
import type { Saved } from "../../data/workspace/save";
import { sizeWord } from "../../data/workspace/size";
import { languageOf } from "../../data/workspace/syntax-languages";
import {
  Banner,
  Button,
  EmptyState,
  Icon,
  IconButton,
  Skeleton,
  TextArea,
  Tooltip,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  FileVersion,
  RelPath,
  SessionId,
  WorkspaceRoot,
} from "../../wire/proto";
import { FileConflict } from "./file-conflict";
import { FileLines } from "./file-lines";
import { useFile, WRITE_LIMIT } from "./use-file";

export function FileView({
  node,
  root,
  session,
  path,
  goTo,
  onBack,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  session: SessionId;
  path: RelPath;
  goTo: { line: number; seq: number } | null;
  onBack: () => void;
}) {
  const file = useFile(node, root, path);
  const [draft, setDraft] = useState<string | null>(null);
  // The version the edit started from: the save's guard, whatever is
  // read after it.
  const [base, setBase] = useState<FileVersion | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<Saved | null>(null);
  const { dir, name } = splitPath(path);
  const language = languageOf(path);
  const text = file.text;
  const editing = draft !== null;
  const dirty =
    editing && (text.kind !== "text" || draft !== text.lines.join("\n"));
  const tooBig = text.kind === "text" && text.version.size > WRITE_LIMIT;
  // The disk moved past what is shown (the file's watch): the view follows
  // it; an open edit keeps its text and says so (Paseo's "Changed on
  // disk"), and its save then meets the change as the conflict below.
  const { disk, refresh } = file;
  useEffect(() => {
    if (disk === "changed" && draft === null) {
      refresh();
    }
  }, [disk, draft, refresh]);

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
  const edit = () => {
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
      <div className="file-head">
        <Tooltip label="All files">
          <IconButton
            label="All files"
            icon="back"
            size="xs"
            variant="ghost"
            onClick={onBack}
          />
        </Tooltip>
        <Icon name="file" size={14} />
        <span className="file-crumb">
          <bdi>
            <span className="d-dir">{dir}</span>
            <span className="d-name">{name}</span>
          </bdi>
        </span>
        {dirty ? (
          <span className="file-dirty" aria-label="Unsaved changes" />
        ) : null}
        <span className="file-head-end">
          {editing ? (
            <>
              <Button
                size="xs"
                variant="ghost"
                icon="close"
                onClick={() => {
                  setDraft(null);
                  setSaved(null);
                }}
              >
                Cancel
              </Button>
              <Button
                size="xs"
                variant="primary"
                busy={saving}
                disabled={!dirty}
                onClick={() => {
                  save(false);
                }}
              >
                Save
              </Button>
            </>
          ) : text.kind === "text" ? (
            <Tooltip label={tooBig ? "File is too large to edit" : "Edit"}>
              <IconButton
                label="Edit"
                icon="edit"
                size="xs"
                variant="ghost"
                disabled={tooBig}
                onClick={edit}
              />
            </Tooltip>
          ) : null}
        </span>
      </div>
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
      ) : text.kind === "binary" ? (
        <EmptyState title="Binary file preview is not available." icon="file">
          {sizeWord(text.version.size)}
        </EmptyState>
      ) : draft !== null ? (
        <div
          className="file-editor"
          onKeyDown={(event) => {
            if (
              (event.metaKey || event.ctrlKey) &&
              event.key.toLowerCase() === "s"
            ) {
              event.preventDefault();
              save(false);
            }
          }}
        >
          <TextArea
            look="bare"
            mono
            fill
            value={draft}
            onChange={setDraft}
            readOnly={saving}
            aria-label={`Edit ${path}`}
          />
        </div>
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
      {disk === "changed" && draft !== null && saved?.kind !== "conflict" ? (
        <div className="file-strip">
          <Banner tone="warn">Changed on disk</Banner>
        </div>
      ) : null}
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
    </div>
  );
}
