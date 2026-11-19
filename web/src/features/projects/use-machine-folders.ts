// The machine's folders for the Add project sheet's field: asked 250 ms
// after the last key (Paseo's debounce), up to 30 (`folder.suggest`,
// ADR 0112), on the pages that search folders; nothing below its protocol
// (plan difference 3). The answer is kept with its machine and text
// (`data/workspace/add-project/folders.ts`).

import { useEffect, useState } from "react";

import type { Fleet } from "../../data/fleet";
import { askFolders } from "../../data/workspace/add-project/calls";
import {
  type FolderAnswer,
  folderAnswer,
  type MachineFolders,
  machineFolders,
} from "../../data/workspace/add-project/folders";
import type { NodeKey } from "../../wire/bridge";
import { HOST_FOLDERS_PROTOCOL } from "../../wire/proto";

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
  const searches = wanted && node !== null && protocol >= HOST_FOLDERS_PROTOCOL;
  const [answer, setAnswer] = useState<FolderAnswer | null>(null);
  useEffect(() => {
    if (!searches) return;
    const asked: NodeKey = node;
    let current = true;
    void askFolders(fleet, asked, debounced).then((outcome) => {
      if (current) setAnswer(folderAnswer(asked, debounced, outcome));
    });
    return () => {
      current = false;
    };
  }, [fleet, node, debounced, searches]);
  return machineFolders(answer, node, query, searches);
}
