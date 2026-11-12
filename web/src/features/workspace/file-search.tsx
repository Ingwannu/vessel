// The Files tab's two searches: a file by name (agent-orchestrator's quick
// open, through `data/workspace/find.ts`, the one wrapper of `file.find`:
// one key for this field, a superseded answer dropped) and lines by their
// content (`file.grep`, its matches grouped under their file). Each result
// opens the file; a line's opens it at that line.

import { Fragment, useEffect, useState } from "react";

import { useFleet } from "../../context";
import { splitPath } from "../../data/workspace/changes";
import { searchFiles } from "../../data/workspace/files";
import { findFiles, newSearchKey } from "../../data/workspace/find";
import { Icon, Spinner } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type {
  FoundFiles,
  GrepMatch,
  GrepResult,
  RelPath,
  WorkspaceRoot,
} from "../../wire/proto";

/** How long typing rests before a search is asked (agent-orchestrator's
 * quick open waits 150ms; the node replaces a search still running). */
const REST_MS = 150;

/** `text` with the characters at `positions` (Unicode scalar values) in
 * bold. */
function Marked({
  text,
  positions,
}: {
  text: string;
  positions: readonly number[];
}) {
  const marked = new Set(positions);
  const chars = Array.from(text);
  return (
    <>
      {chars.map((char, index) =>
        marked.has(index) ? <b key={index}>{char}</b> : char,
      )}
    </>
  );
}

type Answer<T> =
  | { kind: "waiting" }
  | { kind: "ready"; value: T }
  | { kind: "failed"; message: string };

export function FindResults({
  node,
  root,
  query,
  onOpen,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  query: string;
  onOpen: (path: RelPath, line: number | null) => void;
}) {
  const fleet = useFleet();
  const [key] = useState(newSearchKey);
  const [answer, setAnswer] = useState<Answer<FoundFiles>>({ kind: "waiting" });
  useEffect(() => {
    const timer = window.setTimeout(() => {
      void findFiles(fleet, node, root, query, key).then((outcome) => {
        if (!outcome.ok) {
          setAnswer({ kind: "failed", message: outcome.problem.message });
        } else if (outcome.value.superseded !== true) {
          setAnswer({ kind: "ready", value: outcome.value });
        }
      });
    }, REST_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [fleet, node, root, query, key]);
  if (answer.kind === "waiting") {
    return (
      <p className="tree-note-row">
        <Spinner size={12} /> Searching files…
      </p>
    );
  }
  if (answer.kind === "failed") {
    return <p className="tree-note-row failed">{answer.message}</p>;
  }
  const { matches, complete } = answer.value;
  return (
    <div className="file-scroll">
      {matches.length === 0 ? (
        <p className="tree-note-row">No matching files.</p>
      ) : (
        <ul className="found-list" aria-label="Files found">
          {matches.map((match) => {
            const { dir, name } = splitPath(match.path);
            const at = dir.length;
            return (
              <li key={match.path}>
                <button
                  type="button"
                  className="tree-row"
                  onClick={() => {
                    onOpen(match.path, null);
                  }}
                >
                  <Icon name="file" size={14} />
                  <span className="tree-name">
                    <Marked
                      text={name}
                      positions={match.positions
                        .filter((index) => index >= Array.from(dir).length)
                        .map((index) => index - Array.from(dir).length)}
                    />
                  </span>
                  <span className="found-dir">
                    <Marked
                      text={dir.slice(0, Math.max(0, at - 1))}
                      positions={match.positions}
                    />
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
      {complete ? null : (
        <p className="tree-note-row">
          Some paths are not shown. Narrow the search to find more files.
        </p>
      )}
    </div>
  );
}

/** `matches` by file, in the order the node found them. */
function byFile(matches: readonly GrepMatch[]): [RelPath, GrepMatch[]][] {
  const files = new Map<RelPath, GrepMatch[]>();
  for (const match of matches) {
    const list = files.get(match.path) ?? [];
    list.push(match);
    files.set(match.path, list);
  }
  return [...files];
}

export function GrepResults({
  node,
  root,
  query,
  onOpen,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  query: string;
  onOpen: (path: RelPath, line: number | null) => void;
}) {
  const fleet = useFleet();
  const [answer, setAnswer] = useState<Answer<GrepResult>>({ kind: "waiting" });
  useEffect(() => {
    let current = true;
    const timer = window.setTimeout(() => {
      void searchFiles(fleet, node, root, {
        query,
        mode: "literal",
        case: "smart",
      }).then((outcome) => {
        if (current) {
          setAnswer(
            outcome.ok
              ? { kind: "ready", value: outcome.value }
              : { kind: "failed", message: outcome.problem.message },
          );
        }
      });
    }, REST_MS * 2);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [fleet, node, root, query]);
  if (answer.kind === "waiting") {
    return (
      <p className="tree-note-row">
        <Spinner size={12} /> Searching in files…
      </p>
    );
  }
  if (answer.kind === "failed") {
    return <p className="tree-note-row failed">{answer.message}</p>;
  }
  const files = byFile(answer.value.matches);
  return (
    <div className="file-scroll">
      {files.length === 0 ? (
        <p className="tree-note-row">Nothing in these files matches.</p>
      ) : (
        <ul className="found-list" aria-label="Lines found">
          {files.map(([path, matches]) => (
            <Fragment key={path}>
              <li className="grep-file">
                <Icon name="file" size={14} />
                <span className="tree-name">{path}</span>
                <span className="found-dir">{matches.length}</span>
              </li>
              {matches.map((match) => (
                <li
                  key={`${path}:${String(match.line)}:${String(match.column)}`}
                >
                  <button
                    type="button"
                    className="grep-line"
                    onClick={() => {
                      onOpen(path, match.line);
                    }}
                  >
                    <span className="d-num">{match.line}</span>
                    <span className="grep-text">
                      {match.text}
                      {match.cut === true ? " …" : ""}
                    </span>
                  </button>
                </li>
              ))}
            </Fragment>
          ))}
        </ul>
      )}
      {answer.value.more ? (
        <p className="tree-note-row">
          Only the first 500 matches are shown; narrow the search for the rest.
        </p>
      ) : null}
    </div>
  );
}
