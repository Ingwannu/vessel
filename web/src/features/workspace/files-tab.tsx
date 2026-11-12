// The code panel's Files tab (agent-orchestrator's file explorer): a
// filter field at the top that finds a file by name, or, switched, lines
// by their content; the tree under it; a file, once opened, in its place
// with the way back.

import "./files.css";

import { useState } from "react";

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
  session,
  opened,
  onOpened,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  session: SessionId;
  opened: Opened | null;
  onOpened: (opened: Opened | null) => void;
}) {
  const [query, setQuery] = useState("");
  const [content, setContent] = useState(false);
  const [ignored, setIgnored] = useState(false);
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
        key={opened.path}
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
      </div>
      {searching === "" ? (
        <FileTreeView
          key={String(ignored)}
          node={node}
          root={root}
          ignored={ignored}
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
