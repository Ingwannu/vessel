// The Files tab's Modified and Size facts (`file.inspect`) for the folders
// listed so far: which folders to inspect, on the node's background file
// work line (`file-work.ts`), and what a failed inspection leaves.
// `features/workspace/use-entry-facts.ts` holds the facts in the view.
//
// [Decision Log]
// - Purpose: every listed folder ordered by its facts, also with many
//   folders open.
// - Constraint: the node refuses a fifth file work at once (`busy`);
//   inspecting each listed folder together lost a folder's facts with
//   four folders open, and it then sorted last in silence.
// - Choice: the inspections wait on the node's background line, a few at
//   a time; a folder whose inspection fails is forgotten, so the next pass
//   (a folder listed or refreshed, the order changed) asks it again.
// - Cost: a folder refused for another reason (gone, not readable) is
//   asked again at each pass until it is listed no more.

import type { NodeKey } from "../../wire/bridge";
import type { InspectedEntry, RelPath, WorkspaceRoot } from "../../wire/proto";
import type { Fleet } from "../fleet";
import type { Listing } from "./file-tree";
import { inspectDir } from "./files";
import { inBackground } from "./file-work";

/** Inspect each listed folder not yet inspected for its current listing
 * (`asked`: the listing each folder was inspected for, a new listing
 * object being a new look). Each folder's entries go to `found` as they
 * come; the promise settles when every inspection has. */
export async function inspectListed(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  ignored: boolean,
  listings: ReadonlyMap<RelPath, Listing>,
  asked: Map<RelPath, Listing>,
  found: (entries: readonly InspectedEntry[]) => void,
): Promise<void> {
  const inspections: Promise<void>[] = [];
  for (const [dir, listing] of listings) {
    if (listing.kind !== "listed" || asked.get(dir) === listing) continue;
    asked.set(dir, listing);
    inspections.push(
      inBackground(node, () =>
        inspectDir(fleet, node, root, dir, ignored),
      ).then((outcome) => {
        // A slower answer for an older listing of the folder must not
        // overwrite the newer one's facts, nor forget the newer ask.
        if (asked.get(dir) !== listing) return;
        if (!outcome.ok) {
          asked.delete(dir);
          return;
        }
        found(outcome.value.entries);
      }),
    );
  }
  await Promise.all(inspections);
}
