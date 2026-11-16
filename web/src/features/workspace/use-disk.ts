// The disk under a shown file (ADR 0105; Paseo's
// `file-panels/file/handle.ts`, `setObserving` and `receive`): while the
// file is shown, one `file.observe` at a time, anchored on its last
// answer, says whether the disk moved past what the page shows.
//
// [Decision Log]
// - Purpose: a file changed by an agent, git or another editor shows its
//   new content without a reload by hand (`use-file.ts` reads it again).
// - Constraint: the node's observation is a finite call that answers when
//   the file differs from `after`, at its 30 s heartbeat, or at once (a
//   forwarded call); the page must not ask back to back, and its own save
//   comes back as an answer too.
// - Choice: the change rule is `data/workspace/watch.ts` (the native
//   panel's); `pace.ts`'s quiet time is waited out after every answer; a
//   save in flight is waited for before an answer is judged, so the page's
//   own write is never a change (Paseo reports nothing while it writes).
//   A failed observation (an older node, a refusal, the socket gone) ends
//   the watch; the page then behaves as it did before.
// - Cost: one node observer and one wait share per shown file; an answer
//   that arrives during a save is judged up to a write's time later.

import { useCallback, useEffect, useState } from "react";

import { useFleet } from "../../context";
import { observeFile } from "../../data/workspace/files";
import { halted, quiet } from "../../data/workspace/pace";
import { changed } from "../../data/workspace/watch";
import type { NodeKey } from "../../wire/bridge";
import type {
  ContentHash,
  ObservedFile,
  RelPath,
  WorkspaceRoot,
} from "../../wire/proto";

/** The disk against what is shown, as the watch last saw it. */
export type Disk = "same" | "changed";

export interface DiskWatch {
  disk: Disk;
  /** What is shown is the disk again (a read from the start arrived). */
  settled: () => void;
}

/** Follow `path` on disk while it is shown. `shown` gives the hash of
 * what the page shows (null: not known yet, or a file over 8 MiB);
 * `writing` the page's save in flight. Both must keep one identity. */
export function useDisk(
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  shown: () => ContentHash | null,
  writing: () => Promise<unknown> | null,
): DiskWatch {
  const fleet = useFleet();
  const [disk, setDisk] = useState<Disk>("same");

  useEffect(() => {
    const controller = new AbortController();
    const follow = async (signal: AbortSignal) => {
      let after: ObservedFile | null = null;
      while (!halted(signal)) {
        const asked = performance.now();
        const observed = await observeFile(fleet, node, root, path, after);
        if (halted(signal) || !observed.ok) {
          return;
        }
        const saving = writing();
        if (saving !== null) {
          await saving.then(
            () => undefined,
            () => undefined,
          );
          if (halted(signal)) {
            return;
          }
        }
        if (changed(after, observed.value, shown())) {
          setDisk("changed");
        }
        after = observed.value;
        await quiet(asked, signal);
      }
    };
    void follow(controller.signal);
    return () => {
      controller.abort();
    };
  }, [fleet, node, root, path, shown, writing]);

  const settled = useCallback(() => {
    setDisk("same");
  }, []);

  return { disk, settled };
}
