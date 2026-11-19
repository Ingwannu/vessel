// The account's GitHub repositories for the Add project sheet's "Clone
// from GitHub" page (`forge.repos`, ADR 0088; TRACKER 440): one answer,
// kept with the machine and the field text it was asked for, as the
// folders are (`folders.ts`), and the page's words for why the node could
// not search. Paseo's `add-project-flow.tsx` (pinned 0.15.4) lines
// 447-457 (the query), 645-670 (the rows) and 195-200 (the error), and
// `server/src/server/session.ts:7185` for the node's error words, host as
// machine.

import type { NodeKey } from "../../../wire/bridge";
import type {
  ForgeAccess,
  Repository,
  RepositoryList,
} from "../../../wire/proto";
import type { Outcome } from "../../outcome";
import type { GithubRepositoryChoice } from "./model";
import { buildManualGithubRepositoryChoices } from "./options";

/** Paseo's rows per GitHub search. */
export const REPOSITORY_ROWS = 30;

/** One `forge.repos` answer and what it was asked for. */
export interface RepositoryAnswer {
  node: NodeKey;
  query: string;
  choices: readonly GithubRepositoryChoice[];
  /** The call itself failed (Paseo's query error). */
  failed: boolean;
  /** Why the node could not search, in Paseo's words. */
  error: string | null;
}

/** What the page found, for the machine and field as they are now. */
export interface MachineRepositories {
  choices: readonly GithubRepositoryChoice[];
  loading: boolean;
  error: string | null;
}

/** Paseo's words for a search the node could not make: its server's for
 * a missing or signed-out `gh`, else its generic failure. */
export function repositoryAccessError(access: ForgeAccess): string | null {
  if (access.access === "ready") return null;
  if (access.access !== "signed_out" || access.gh === "failed") {
    return "GitHub search failed";
  }
  if (access.gh === "missing") {
    return "GitHub CLI (gh) is not installed or not in PATH";
  }
  return "GitHub CLI is not authenticated. Run gh auth login on the machine.";
}

/** A search row as a choice: its owner/name as the title (no protocol:
 * the node's clone URL is used as it is). */
export function repositoryChoice(row: Repository): GithubRepositoryChoice {
  return {
    id: `repository:${row.path}`,
    nameWithOwner: row.path,
    cloneUrl: row.clone_url,
    description: row.description ?? null,
    updatedAt: row.updated_at,
  };
}

/** `outcome`, the answer to `query` on `node`, as an answer. */
export function repositoryAnswer(
  node: NodeKey,
  query: string,
  outcome: Outcome<RepositoryList>,
): RepositoryAnswer {
  if (!outcome.ok) {
    return { node, query, choices: [], failed: true, error: null };
  }
  return {
    node,
    query,
    choices: outcome.value.repositories.map(repositoryChoice),
    failed: false,
    error: repositoryAccessError(outcome.value.access),
  };
}

/** The page's repositories: `answer`'s when it was asked on `node` for
 * `query`, otherwise none, loading while the page `searches`. */
export function machineRepositories(
  answer: RepositoryAnswer | null,
  node: NodeKey | null,
  query: string,
  searches: boolean,
): MachineRepositories {
  if (!searches || answer?.node !== node || answer.query !== query) {
    return { choices: [], loading: searches, error: null };
  }
  return {
    choices: answer.choices,
    loading: false,
    error: answer.failed
      ? "Unable to search GitHub repositories"
      : answer.error,
  };
}

/** Paseo's GitHub rows: the typed choices first, unless the search
 * already has that repository, then the search's. */
export function githubChoices(
  query: string,
  repositories: readonly GithubRepositoryChoice[],
): GithubRepositoryChoice[] {
  const normalized = query.trim().toLowerCase();
  const exact = repositories.some(
    (repository) =>
      repository.nameWithOwner.toLowerCase() === normalized ||
      repository.cloneUrl.toLowerCase() === normalized,
  );
  const typed = exact ? [] : buildManualGithubRepositoryChoices(query);
  return [...typed, ...repositories];
}
