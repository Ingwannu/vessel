// Labels across machines: one list for the surfaces that span them (the
// view menu's filter, the manager's machine list), and each workspace's
// chips from its own machine only.
//
// [Decision Log]
// - Purpose: two machines may both have "Blocked", maybe in two colors.
//   The filter shows it once; a chip must not borrow the other machine's
//   color.
// - Reference: Paseo merges host catalogs with the target host first, then
//   by host id, first definition per key wins, sorted by name; a
//   workspace's chips resolve against its own host, and a name its catalog
//   does not know is left out rather than drawn in an invented color.
// - Choice: the same rules, with the node key as the host id.

import type { NodeKey } from "../../wire/bridge";
import type { Label } from "../../wire/proto";
import { byLabelName, labelKey } from "./names";

/** One machine's catalog. */
export interface NodeCatalog {
  node: NodeKey;
  labels: readonly Label[];
}

/** Every label of `catalogs` once, by name; on a name two machines share,
 * `target`'s definition, else the machine whose key sorts first. */
export function mergeCatalogs(
  catalogs: readonly NodeCatalog[],
  target: NodeKey | null = null,
): Label[] {
  const ordered = [...catalogs].sort((left, right) => {
    if (left.node === target) {
      return -1;
    }
    if (right.node === target) {
      return 1;
    }
    return left.node.localeCompare(right.node);
  });
  const merged = new Map<string, Label>();
  for (const catalog of ordered) {
    for (const label of catalog.labels) {
      const key = labelKey(label.name);
      if (!merged.has(key)) {
        merged.set(key, label);
      }
    }
  }
  return [...merged.values()].sort((left, right) =>
    byLabelName(left.name, right.name),
  );
}

/** The definitions behind `names`, in assignment order, from the
 * workspace's own machine's `catalog`; a name it does not know (its first
 * read is still under way) is left out. */
export function chipsOf(
  catalog: readonly Label[],
  names: readonly string[],
): Label[] {
  const known = new Map(catalog.map((label) => [labelKey(label.name), label]));
  return names.flatMap((name) => {
    const label = known.get(labelKey(name));
    return label === undefined ? [] : [label];
  });
}
