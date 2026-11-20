// The code panel's Files tab (agent-orchestrator's file explorer): a
// filter field at the top that finds a file by name, or, switched, lines
// by their content; the tree under it; a file, once opened, in its place
// with the way back.

import "./files.css";

import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { nextSort, SORT_WORDS } from "../../data/workspace/file-tree";
import { hasFilePanels } from "../../data/workspace/files";
import { Button, Icon, IconButton, TextField, Tooltip } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, SessionId, WorkspaceRoot } from "../../wire/proto";
import { FindResults, GrepResults } from "./file-search";
import { FileTreeView } from "./file-tree-view";

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
  onOpen,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  /** The root's folder on the node, when known (the Explorer's Copy
   * path). */
  folder: string | null;
  session: SessionId;
  onOpen: (path: RelPath, line: number | null) => void;
}) {
  const fleet = useFleet();
  const protocol = useFleetState((state) => {
    const status = state.statuses[node];
    return status?.state === "connected" ? status.protocol : null;
  });
  // Paseo's explorer sort, kept with the page's panels (the bridge's
  // `layout.explorer_sort`); Modified and Size need `file.inspect`, so a
  // node older than the file panels protocol sorts by name.
  const kept = useStore(prefs, (state) => state.values.layout.explorer_sort);
  const sortable = hasFilePanels(protocol);
  const sort = sortable ? kept : "name";
  const [query, setQuery] = useState("");
  const [content, setContent] = useState(false);
  const [ignored, setIgnored] = useState(false);
  // The tree's open folders, kept here so a file opened and closed comes
  // back to the tree as it was; `round` reads them all again (Refresh).
  const [folders, setFolders] = useState<ReadonlySet<RelPath>>(() => new Set());
  const [round, setRound] = useState(0);
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
        {sortable && searching === "" ? (
          // Paseo's sort trigger: the order's word and a chevron; a press
          // moves to the next order.
          <Tooltip label="Sort">
            <Button
              size="xs"
              variant="ghost"
              aria-label={`Sort: ${SORT_WORDS[sort]}`}
              end={<Icon name="chevron-down" size={12} />}
              onClick={() => {
                void prefer(fleet, {
                  set: "explorer_sort",
                  value: nextSort(sort),
                });
              }}
            >
              {SORT_WORDS[sort]}
            </Button>
          </Tooltip>
        ) : null}
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
          sort={sort}
          open={folders}
          onOpenChange={setFolders}
          onFile={(path) => {
            onOpen(path, null);
          }}
        />
      ) : content ? (
        <GrepResults
          key={searching}
          node={node}
          root={root}
          query={searching}
          onOpen={onOpen}
        />
      ) : (
        <FindResults
          node={node}
          root={root}
          query={searching}
          onOpen={onOpen}
        />
      )}
    </div>
  );
}
