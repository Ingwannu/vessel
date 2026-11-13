// The files `@` finds as its word changes: through the one wrapper of
// `file.find` (data/workspace/find.ts), one key for the composer's life,
// 80ms after the word changes and only once it has a character (the node
// refuses an empty query). An answer for an older word is dropped; one
// `superseded` with no newer search of this composer's (another tab's
// search took the key: every tab starts its keys at 1) is asked again,
// once.

import { useEffect, useMemo, useRef, useState } from "react";

import { useFleet } from "../../context";
import { MAX_ROWS } from "../../data/suggest";
import { findFiles, newSearchKey } from "../../data/workspace/find";
import type { NodeKey } from "../../wire/bridge";
import type { FoundFile, WorkspaceRoot } from "../../wire/proto";

/** What the `@` list holds for a word. */
export type Found =
  | { query: string; kind: "searching" }
  | {
      query: string;
      kind: "found";
      files: readonly FoundFile[];
      complete: boolean;
    }
  | { query: string; kind: "failed"; message: string };

/** The files found for `query` under `root`; `null` before a search. */
export function useFileSearch(
  node: NodeKey,
  root: WorkspaceRoot | null,
  query: string | null,
): Found | null {
  const fleet = useFleet();
  const [found, setFound] = useState<Found | null>(null);
  const key = useMemo(() => newSearchKey(), []);
  const asked = useRef(0);
  useEffect(() => {
    if (query === null || root === null || query.trim() === "") {
      return undefined;
    }
    const wanted = query;
    const timer = window.setTimeout(() => {
      asked.current += 1;
      const mine = asked.current;
      setFound((current) =>
        current?.query === wanted
          ? current
          : { query: wanted, kind: "searching" },
      );
      const ask = (again: boolean) => {
        void findFiles(fleet, node, root, wanted, key).then((outcome) => {
          if (mine !== asked.current) {
            return;
          }
          if (!outcome.ok) {
            setFound({
              query: wanted,
              kind: "failed",
              message: outcome.problem.message,
            });
          } else if (outcome.value.superseded === true) {
            if (again) {
              ask(false);
            }
          } else {
            setFound({
              query: wanted,
              kind: "found",
              files: outcome.value.matches.slice(0, MAX_ROWS),
              complete: outcome.value.complete,
            });
          }
        });
      };
      ask(true);
    }, 80);
    return () => {
      window.clearTimeout(timer);
    };
  }, [fleet, node, root, key, query]);
  return found;
}
