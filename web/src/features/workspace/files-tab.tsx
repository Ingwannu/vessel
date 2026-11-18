// The code panel's Files tab (agent-orchestrator's file explorer): a
// filter field at the top that finds a file by name, or, switched, lines
// by their content; the tree under it; a file, once opened, in its place
// with the way back.

import "./files.css";

import { useState } from "react";

import { rootKey } from "../../data/workspace/root";
import { IconButton, TextField, Tooltip } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionId, WorkspaceRoot } from "../../wire/proto";
import { FindResults, GrepResults } from "./file-search";
import { FileTreeView } from "./file-tree-view";
import { FileView } from "./file-view";

/** The file open in the tab, and the line asked for. */
export interface Opened {
  path: RelPath;
  goTo: { line: number; seq: number } | null;
}

export function FilesTab({
  node,
  root,
  folder,
  session,
  opened,
  onOpened,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  /** The root's folder on the node, when known (the Explorer's Copy
   * path). */
  folder: string | null;
  session: SessionId;
  opened: Opened | null;
  onOpened: (opened: Opened | null) => void;
}) {
  const [query, setQuery] = useState("");
  const [content, setContent] = useState(false);
  const [ignored, setIgnored] = useState(false);
  // The tree's open folders, kept here so a file opened and closed comes
  // back to the tree as it was; `round` reads them all again (Refresh).
  const [folders, setFolders] = useState<ReadonlySet<RelPath>>(() => new Set());
  const [round, setRound] = useState(0);
  const [seq, setSeq] = useState(0);
  const open = (path: RelPath, line: number | null) => {
    setSeq((value) => value + 1);
    onOpened({
      path,
      goTo: line === null ? null : { line, seq: seq + 1 },
    });
  };
  if (opened !== null) {
    return (
      <FileView
        // One view per root and file: its reads, edit and disk watch
        // belong to that file only.
        key={`${rootKey(root)} ${opened.path}`}
        node={node}
        root={root}
        session={session}
        path={opened.path}
        goTo={opened.goTo}
        onBack={() => {
          onOpened(null);
        }}
      />
    );
  }
  const searching = query.trim();
  return (
    <div className="files">
      <div className="files-tools">
        <span className="files-filter">
          <TextField
            value={query}
            onChange={setQuery}
            size="sm"
            icon="search"
            placeholder={content ? "Search in files" : "Go to file"}
            aria-label={content ? "Search in files" : "Go to file"}
            role="searchbox"
          />
        </span>
        <Tooltip label={content ? "Find a file by name" : "Search in files"}>
          <IconButton
            label="Search in files"
            icon="list"
            size="xs"
            variant="ghost"
            aria-pressed={content}
            onClick={() => {
              setContent(!content);
            }}
          />
        </Tooltip>
        <Tooltip label={ignored ? "Hide ignored files" : "Show ignored files"}>
          <IconButton
            label="Show ignored files"
            icon="circle-dashed"
            size="xs"
            variant="ghost"
            aria-pressed={ignored}
            onClick={() => {
              setIgnored(!ignored);
            }}
          />
        </Tooltip>
        <Tooltip label="Refresh">
          <IconButton
            label="Refresh files"
            icon="refresh"
            size="xs"
            variant="ghost"
            onClick={() => {
              setRound((value) => value + 1);
            }}
          />
        </Tooltip>
      </div>
      {searching === "" ? (
        <FileTreeView
          key={`${String(ignored)}:${String(round)}`}
          node={node}
          root={root}
          folder={folder}
          session={session}
          ignored={ignored}
          open={folders}
          onOpenChange={setFolders}
          onFile={(path) => {
            open(path, null);
          }}
        />
      ) : content ? (
        <GrepResults
          key={searching}
          node={node}
          root={root}
          query={searching}
          onOpen={open}
        />
      ) : (
        <FindResults node={node} root={root} query={searching} onOpen={open} />
      )}
    </div>
  );
}
