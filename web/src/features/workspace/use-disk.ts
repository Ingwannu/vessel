// The disk under a shown file (ADR 0105; Paseo's
// `file-panels/file/handle.ts`, `setObserving` and `receive`): while the
// file is shown, one `file.observe` at a time, anchored on its last
// answer, says whether the disk moved past what the page shows, or the
// file is gone.
//
// [Decision Log]
// - Purpose: a file changed by an agent, git or another editor shows its
//   new content without a reload by hand (`use-file.ts` reads it again);
//   an open edit says the disk moved (Paseo's conflict alert).
// - Constraint: the node's observation is a finite call that answers when
//   the file differs from `after`, at its 30 s heartbeat, or at once (a
//   forwarded call); the page must not ask back to back, and its own save
//   comes back as an answer too. The bridge has no cancel: a call the page
//   no longer wants runs on at the node until it answers.
// - Choice: the rule is `data/workspace/watch.ts` (the native panel's).
//   The watch starts once the file's first read has landed, so the first
//   answer is always judged against what is shown (an answer before it
//   could anchor a newer disk the page never read). `pace.ts`'s quiet time
//   is waited out after every answer; a save in flight is waited for
//   before an answer is judged, so the page's own write is never a change
//   (Paseo reports nothing while it writes). The node's `unavailable` (its
//   observers all busy) is asked again after the quiet time; any other
//   failure (an older node, a refusal, the socket gone) ends the watch and
//   the page behaves as it did before.
// - Cost: one node observer and one wait share of the page's origin per
//   shown file. Leaving the file ignores its call, which holds both until
//   the node's heartbeat; flipping through many files within 30 s turns the
//   origin's other waiting reads into paced polls for that while.

import { useCallback, useEffect, useState } from "react";

import { useFleet } from "../../context";
import { observeFile } from "../../data/workspace/files";
import { halted, quiet } from "../../data/workspace/pace";
import { nodeError } from "../../data/workspace/root";
import { type Disk, diskAfter } from "../../data/workspace/watch";
import type { NodeKey } from "../../wire/bridge";
import type {
  ContentHash,
  ObservedFile,
  RelPath,
  WorkspaceRoot,
} from "../../wire/proto";

export type { Disk } from "../../data/workspace/watch";

/** A promise kept once, by `keep`. */
interface Latch {
  kept: Promise<void>;
  keep: () => void;
}

function latch(): Latch {
  let keep: () => void = () => undefined;
  const kept = new Promise<void>((resolve) => {
    keep = resolve;
  });
  return { kept, keep };
}

export interface DiskWatch {
  disk: Disk;
  /** What is shown is the disk again (a read from the start arrived). */
  settled: () => void;
}

/** Follow `path` on disk while it is shown, from the moment `read` (the
 * file's first read landed) holds. `shown` gives the hash of what the
 * page shows (null: a file over 8 MiB); `writing` the page's save in
 * flight. Both must keep one identity. */
export function useDisk(
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  read: boolean,
  shown: () => ContentHash | null,
  writing: () => Promise<unknown> | null,
): DiskWatch {
  const fleet = useFleet();
  const [disk, setDisk] = useState<Disk>("same");
  // Kept by the first read: a later reload or a failed read does not stop
  // the watch (a deleted file must be seen coming back).
  const [firstRead] = useState(latch);
  useEffect(() => {
    if (read) {
      firstRead.keep();
    }
  }, [read, firstRead]);

  useEffect(() => {
    const controller = new AbortController();
    const follow = async (signal: AbortSignal) => {
      await firstRead.kept;
      let after: ObservedFile | null = null;
      while (!halted(signal)) {
        const asked = performance.now();
        const observed = await observeFile(fleet, node, root, path, after);
        if (halted(signal)) {
          return;
        }
        if (!observed.ok) {
          if (nodeError(observed.problem) !== "unavailable") {
            return;
          }
          await quiet(asked, signal);
          continue;
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
        const before = after;
        const now = shown();
        setDisk((was) => diskAfter(was, before, observed.value, now));
        after = observed.value;
        await quiet(asked, signal);
      }
    };
    void follow(controller.signal);
    return () => {
      controller.abort();
    };
  }, [fleet, node, root, path, firstRead, shown, writing]);

  const settled = useCallback(() => {
    setDisk("same");
  }, []);

  return { disk, settled };
}
