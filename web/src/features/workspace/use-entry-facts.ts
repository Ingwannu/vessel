// The facts the Files tab's Modified and Size orders need (Paseo's
// explorer entries carry `modifiedAt` and `size`): each listed folder
// inspected once per listing (`file.inspect`), only while one of those
// orders is chosen.
//
// [Decision Log]
// - Purpose: the Modified sort (TRACKER: Paseo's explorer sort cycle,
//   ADR 0105's "later work").
// - Constraint: `file.list` gives names and kinds, not times; the node's
//   `file.inspect` (protocol 27) gives both for one folder. Name needs
//   neither, so a tree in Name order asks nothing more of the node.
// - Choice: a folder is inspected when its listing arrives (or arrives
//   again: a refresh, a change made from the menu) and the order needs it,
//   waiting on the node's background file work line
//   (`data/workspace/entry-facts.ts`); a folder whose inspection fails
//   keeps no facts and its entries sort last, as entries the node gave no
//   time for, until the next pass asks it again.
// - Cost: one more call per folder shown while Modified or Size is chosen.

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { inspectListed } from "../../data/workspace/entry-facts";
import type { EntryFacts, Listing } from "../../data/workspace/file-tree";
import type { ExplorerSort, NodeKey } from "../../wire/bridge";
import type { RelPath, WorkspaceRoot } from "../../wire/proto";

/** Each entry's facts by path, for the folders listed so far. */
export function useEntryFacts(
  node: NodeKey,
  root: WorkspaceRoot,
  ignored: boolean,
  listings: ReadonlyMap<RelPath, Listing>,
  sort: ExplorerSort,
): ReadonlyMap<RelPath, EntryFacts> {
  const fleet = useFleet();
  const [facts, setFacts] = useState<ReadonlyMap<RelPath, EntryFacts>>(
    () => new Map(),
  );
  // The listing each folder was inspected for: a new listing object is a
  // new look at the folder.
  const asked = useRef(new Map<RelPath, Listing>());
  useEffect(() => {
    if (sort === "name") return;
    void inspectListed(
      fleet,
      node,
      root,
      ignored,
      listings,
      asked.current,
      (entries) => {
        setFacts((known) => {
          const next = new Map(known);
          for (const entry of entries) {
            next.set(entry.path, {
              modified: entry.modified,
              size: entry.size,
            });
          }
          return next;
        });
      },
    );
  }, [fleet, node, root, ignored, listings, sort]);
  return facts;
}
