// One machine's label catalog as the page holds it (`workspace.label.list`,
// protocol 28): a snapshot, then the changes after it.
//
// [Decision Log]
// - Purpose: chips draw a label in its catalog color, the picker lists
//   the catalog, and the manager edits it. The page keeps each machine's
//   catalog current without reading it whole after every change.
// - Reference: Paseo's host replica applies a list, then each update whose
//   sequence follows the one it holds, and refuses anything else so the
//   caller lists again.
// - Choice: the node answers an observing read with either a snapshot or
//   the changes after a cursor. A change set that does not start at the
//   cursor held here is a gap; `apply` refuses it and the caller reads a
//   snapshot instead of drawing a catalog that skipped a rename.

import type { CatalogCursor, CatalogList, Label } from "../../wire/proto";
import { byLabelName, labelKey } from "./names";

function sameCursor(left: CatalogCursor, right: CatalogCursor): boolean {
  return left.generation === right.generation && left.after === right.after;
}

export class CatalogReplica {
  #labels = new Map<string, Label>();
  #head: CatalogCursor | null = null;

  /** Where the next observing read continues from; `null` before the
   * first snapshot. */
  get head(): CatalogCursor | null {
    return this.#head;
  }

  /** The labels, by name. */
  labels(): Label[] {
    return [...this.#labels.values()].sort((left, right) =>
      byLabelName(left.name, right.name),
    );
  }

  /** Take `list` in. `false` when its changes do not follow what is held:
   * nothing changed, and the caller reads a snapshot. */
  apply(list: CatalogList): boolean {
    const data = list.data;
    if (data.mode === "snapshot") {
      this.#labels = new Map(
        data.labels.map((label) => [labelKey(label.name), label]),
      );
      this.#head = list.head;
      return true;
    }
    if (this.#head === null || !sameCursor(data.from, this.#head)) {
      return false;
    }
    const next = new Map(this.#labels);
    for (const { delta } of data.updates) {
      if (delta.kind === "remove") {
        next.delete(labelKey(delta.name));
      } else {
        if (delta.previous !== null) {
          next.delete(labelKey(delta.previous));
        }
        next.set(labelKey(delta.label.name), delta.label);
      }
    }
    this.#labels = next;
    this.#head = list.head;
    return true;
  }

  /** Forget everything, so the next read asks for a snapshot. */
  reset(): void {
    this.#labels = new Map();
    this.#head = null;
  }
}
