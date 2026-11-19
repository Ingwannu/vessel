// The machine's folders for the Add project sheet's field
// (`folder.suggest`, ADR 0112): one answer, kept with the machine and the
// field text it was asked for, and what the field shows from it.
//
// An answer counts only for the machine and the text it was asked for.
// Keyed by the text alone, switching machine with the same text showed
// the first machine's folders as the second's rows until the second
// answered, and Enter then added the first machine's path on the second
// (node review of TRACKER 234, R2).

import type { NodeKey } from "../../../wire/bridge";
import type { FolderSuggestions } from "../../../wire/proto";
import type { Outcome } from "../../outcome";

/** One `folder.suggest` answer and what it was asked for. */
export interface FolderAnswer {
  node: NodeKey;
  query: string;
  paths: readonly string[];
  /** The node's `~` form of each path. */
  shown: ReadonlyMap<string, string>;
  failed: boolean;
}

/** What the field found, for the machine and field as they are now. */
export interface MachineFolders {
  paths: readonly string[];
  /** The node's `~` form of each path. */
  shown: ReadonlyMap<string, string>;
  loading: boolean;
  /** Paseo's "Unable to search directories", or none. */
  error: string | null;
}

const NO_PATHS: ReadonlyMap<string, string> = new Map();

/** `outcome`, the answer to `query` on `node`, as an answer. */
export function folderAnswer(
  node: NodeKey,
  query: string,
  outcome: Outcome<FolderSuggestions>,
): FolderAnswer {
  if (!outcome.ok) {
    return { node, query, paths: [], shown: NO_PATHS, failed: true };
  }
  const folders = outcome.value.folders;
  return {
    node,
    query,
    paths: folders.map((folder) => folder.path),
    shown: new Map(folders.map((folder) => [folder.path, folder.shown])),
    failed: false,
  };
}

/** The field's folders: `answer`'s when it was asked on `node` for
 * `query`, otherwise none, loading while the page `searches`. */
export function machineFolders(
  answer: FolderAnswer | null,
  node: NodeKey | null,
  query: string,
  searches: boolean,
): MachineFolders {
  if (!searches || answer?.node !== node || answer.query !== query) {
    return { paths: [], shown: NO_PATHS, loading: searches, error: null };
  }
  return {
    paths: answer.paths,
    shown: answer.shown,
    loading: false,
    error: answer.failed ? "Unable to search directories" : null,
  };
}
