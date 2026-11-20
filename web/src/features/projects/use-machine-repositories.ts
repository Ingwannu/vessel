// The account's GitHub repositories for the Add project sheet's "Clone
// from GitHub" page: asked 250 ms after the last key (Paseo's debounce),
// up to 30 (`forge.repos`, ADR 0088), only on that page and on a machine
// that searches (TRACKER 440). The answer is kept with its machine and
// text (`data/workspace/add-project/repositories.ts`), as the folders are
// (`use-machine-folders.ts`).

import { useEffect, useState } from "react";

import type { Fleet } from "../../data/fleet";
import { askRepositories } from "../../data/workspace/add-project/calls";
import {
  type MachineRepositories,
  machineRepositories,
  type RepositoryAnswer,
  repositoryAnswer,
} from "../../data/workspace/add-project/repositories";
import type { NodeKey } from "../../wire/bridge";

export function useMachineRepositories(
  fleet: Fleet,
  node: NodeKey | null,
  query: string,
  /** The GitHub search page is shown and its machine searches. */
  searches: boolean,
): MachineRepositories {
  const [debounced, setDebounced] = useState(query);
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebounced(query);
    }, 250);
    return () => {
      clearTimeout(timer);
    };
  }, [query]);
  const asks = searches && node !== null;
  const [answer, setAnswer] = useState<RepositoryAnswer | null>(null);
  useEffect(() => {
    if (!asks) return;
    const asked: NodeKey = node;
    let current = true;
    void askRepositories(fleet, asked, debounced).then((outcome) => {
      if (current) setAnswer(repositoryAnswer(asked, debounced, outcome));
    });
    return () => {
      current = false;
    };
  }, [fleet, node, debounced, asks]);
  return machineRepositories(answer, node, query, asks);
}
