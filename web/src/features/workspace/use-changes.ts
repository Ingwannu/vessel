// What the Changes tab shows, kept as it is asked for: the comparison's
// files and the repository's status, read again every few seconds while
// the page is in front (an agent changes files as it works), and each
// file's diff read when its place in the list is drawn, a page at a time.
//
// [Decision Log]
// - Purpose: a review pane that keeps up with an agent at work, without
//   reading every diff of a large change.
// - Constraint: the node sends no event when files change (track B adds
//   none); a summary is at most 5,000 files, a diff 5,000 lines a page.
// - Alternatives: a refresh button only (agent-orchestrator refreshes on
//   its daemon's events, which Vessel's node does not send for files);
//   reading every open file's diff on each refresh (one git run per file,
//   every few seconds).
// - Choice: the summary and status every `EVERY_MS` while the document is
//   visible, a diff kept while its file's status and counts stay the
//   same, and read again (when drawn) once they change.
// - Cost: two git runs on the node every five seconds per open panel.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { Problem } from "../../data/outcome";
import {
  type FileState,
  rowPrefix,
  sameChange,
} from "../../data/workspace/changes";
import { addPage, type DiffBody } from "../../data/workspace/diff-rows";
import {
  readChanges,
  readFileDiff,
  readStatus,
} from "../../data/workspace/git";
import type { NodeKey } from "../../wire/bridge";
import type {
  DiffScope,
  DiffSummary,
  GitStatus,
  RelPath,
  WorkspaceRoot,
} from "../../wire/proto";

/** How often the comparison is read again while the page is in front. */
const EVERY_MS = 5000;

/** The comparison as last read. */
export type Summary =
  | { kind: "loading" }
  | { kind: "ready"; summary: DiffSummary }
  | { kind: "failed"; problem: Problem };

/** A file's diff read so far, when it has rows and more of them. */
export type ReadRows = Extract<DiffBody, { kind: "rows" }>;

export interface Changes {
  summary: Summary;
  status: GitStatus | null;
  states: ReadonlyMap<RelPath, FileState>;
  /** Read the comparison and the status now. */
  refresh: () => void;
  /** Read a file's diff when its row is drawn: its first page, or the
   * page after `after`. */
  want: (path: RelPath, after: ReadRows | null) => void;
  /** Take a status a write answered with, and read the rest again. */
  took: (status: GitStatus) => void;
}

export function useChanges(
  node: NodeKey,
  root: WorkspaceRoot,
  scope: DiffScope,
  ignoreWhitespace: boolean,
): Changes {
  const fleet = useFleet();
  const [summary, setSummary] = useState<Summary>({ kind: "loading" });
  const [status, setStatus] = useState<GitStatus | null>(null);
  const [states, setStates] = useState<ReadonlyMap<RelPath, FileState>>(
    () => new Map(),
  );
  // Answers to an older comparison are dropped: each read carries the
  // generation it was asked in.
  const generation = useRef(0);
  const reading = useRef(false);
  // The files as last read, to keep a diff while its file is the same.
  const last = useRef<DiffSummary | null>(null);
  // Diff pages asked for and not answered yet, so a row drawn again does
  // not ask twice.
  const inflight = useRef(new Set<string>());

  const read = useCallback(() => {
    if (reading.current) {
      return;
    }
    reading.current = true;
    const asked = generation.current;
    void Promise.all([
      readChanges(fleet, node, root, scope, ignoreWhitespace),
      readStatus(fleet, node, root),
    ]).then(([changes, current]) => {
      reading.current = false;
      if (asked !== generation.current) {
        return;
      }
      if (current.ok) {
        setStatus(current.value);
      }
      if (!changes.ok) {
        setSummary({ kind: "failed", problem: changes.problem });
        return;
      }
      const old = new Map(
        (last.current?.files ?? []).map((file) => [file.path, file]),
      );
      last.current = changes.value;
      setStates((known) => {
        const kept = new Map<RelPath, FileState>();
        for (const file of changes.value.files) {
          const was = old.get(file.path);
          const state = known.get(file.path);
          if (
            was !== undefined &&
            state !== undefined &&
            sameChange(was, file)
          ) {
            kept.set(file.path, state);
          }
        }
        return kept;
      });
      setSummary({ kind: "ready", summary: changes.value });
    });
  }, [fleet, node, root, scope, ignoreWhitespace]);

  // The caller keys this hook's component by its root and comparison,
  // so a new comparison is a new mount and starts from "loading".
  useEffect(() => {
    generation.current += 1;
    reading.current = false;
    read();
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        read();
      }
    }, EVERY_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [read]);

  const want = useCallback(
    (path: RelPath, after: ReadRows | null) => {
      const from = after?.more ?? null;
      const ask = `${path}\u0000${String(from)}`;
      if (inflight.current.has(ask) || (after !== null && from === null)) {
        return;
      }
      inflight.current.add(ask);
      const asked = generation.current;
      setStates((known) =>
        new Map(known).set(path, { kind: "loading", body: after }),
      );
      void readFileDiff(fleet, node, {
        root,
        scope,
        path,
        ignoreWhitespace,
        fromHunk: from,
      }).then((page) => {
        inflight.current.delete(ask);
        if (asked !== generation.current) {
          return;
        }
        setStates((known) =>
          new Map(known).set(
            path,
            page.ok
              ? {
                  kind: "read",
                  body: addPage(after, page.value, rowPrefix(path)),
                }
              : { kind: "failed", message: page.problem.message },
          ),
        );
      });
    },
    [fleet, node, root, scope, ignoreWhitespace],
  );

  const took = useCallback(
    (next: GitStatus) => {
      setStatus(next);
      read();
    },
    [read],
  );

  return { summary, status, states, refresh: read, want, took };
}
