// The machine's folders for the Add project sheet's field: asked 250 ms
// after the last key (Paseo's debounce), up to 30 (`folder.suggest`,
// ADR 0112), on the pages that search folders; nothing below its protocol
// (plan difference 3).

import { useEffect, useState } from "react";

import type { Fleet } from "../../data/fleet";
import {
  askFolders,
  FOLDERS_PROTOCOL,
} from "../../data/workspace/add-project/calls";
import type { NodeKey } from "../../wire/bridge";

interface Answer {
  query: string;
  paths: string[];
  shown: ReadonlyMap<string, string>;
  failed: boolean;
}

const NONE: Answer = { query: "", paths: [], shown: new Map(), failed: false };

/** What the field found, for the field as it is now. */
export interface MachineFolders {
  paths: readonly string[];
  /** The node's `~` form of each path. */
  shown: ReadonlyMap<string, string>;
  loading: boolean;
  /** Paseo's "Unable to search directories", or none. */
  error: string | null;
}

export function useMachineFolders(
  fleet: Fleet,
  node: NodeKey | null,
  query: string,
  /** Whether the page searches folders (and the machine can). */
  wanted: boolean,
  protocol: number,
): MachineFolders {
  const [debounced, setDebounced] = useState(query);
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebounced(query);
    }, 250);
    return () => {
      clearTimeout(timer);
    };
  }, [query]);
  const searches = wanted && node !== null && protocol >= FOLDERS_PROTOCOL;
  const [answer, setAnswer] = useState<Answer>(NONE);
  useEffect(() => {
    if (!searches) return;
    let current = true;
    void askFolders(fleet, node, debounced).then((outcome) => {
      if (!current) return;
      setAnswer(
        outcome.ok
          ? {
              query: debounced,
              paths: outcome.value.folders.map((folder) => folder.path),
              shown: new Map(
                outcome.value.folders.map((folder) => [
                  folder.path,
                  folder.shown,
                ]),
              ),
              failed: false,
            }
          : { ...NONE, query: debounced, failed: true },
      );
    });
    return () => {
      current = false;
    };
  }, [fleet, node, debounced, searches]);
  // The answer counts only for the field as it is; until then, loading.
  const answered = searches && answer.query === query;
  return {
    paths: answered ? answer.paths : [],
    shown: answered ? answer.shown : NONE.shown,
    loading: searches && !answered,
    error: answered && answer.failed ? "Unable to search directories" : null,
  };
}
