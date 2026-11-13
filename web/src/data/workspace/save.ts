// Saving an edited file: the guard a save carries, and what its answer
// means for the editor (ADR 0027's amendment of wave 2's join).
//
// [Decision Log]
// - Purpose: a save must never overwrite a change it did not see (an
//   agent's, another editor's), and when it finds one it must show both
//   states, keeping the edit, so a person chooses (GOAT's banner: reload
//   from disk or overwrite).
// - Constraint: `file.write` (protocol 11) is guarded by the SHA-256 of
//   the content the edit started from (`expect: { kind: "content", hash
//   }`), which the node compares on its side and refuses with `conflict`,
//   writing nothing. A file over 8 MiB carries no hash; the editor edits
//   at most 1 MiB, so an editable file always has one. Protocol 10's
//   guard compared a nanosecond time no JavaScript number holds, and the
//   page then checked the disk itself and wrote with `any`, which lost a
//   change landing between its check and its write.
// - Alternatives: keep the page's own check before an `any` (the window
//   above, which the node's guard closes); send `any` when the version
//   has no hash (an unguarded write a person did not ask for); retry a
//   conflict whose disk matches the edit's start (the node already
//   compared exactly that).
// - Choice: the guard is the hash, or nothing is written and the editor
//   says why; `any` only when the person pressed Overwrite. A `conflict`
//   is read back once (the disk's version and first page) for the banner,
//   and is never followed by another write.
// - Cost: one more read after a refused save; none after a save that
//   goes through.

import type { FileRead, FileVersion, WriteExpect } from "../../wire/proto";
import type { Outcome } from "../outcome";
import { nodeError } from "./root";

/** A save's answer. */
export type Saved =
  /** Written; the file is now `version`. */
  | { kind: "saved"; version: FileVersion }
  /** The file is not what the edit started from; nothing was written. */
  | {
      kind: "conflict";
      /** The disk's version now, `null` when it is gone or unreadable. */
      disk: FileVersion | null;
      /** The disk's text, when it is text read whole in one page. */
      diskText: string | null;
      /** The text that was refused. */
      mine: string;
    }
  /** Not written, for another reason (said by the node or the page). */
  | { kind: "failed"; message: string };

/** What a save of an edit that started from `base` sends as its guard;
 * `null` when it can carry none (no hash: the file was over 8 MiB when
 * read). `base` `null` is the person's Overwrite. */
export function saveGuard(base: FileVersion | null): WriteExpect | null {
  if (base === null) {
    return { kind: "any" };
  }
  const hash = base.hash ?? null;
  return hash === null ? null : { kind: "content", hash };
}

/** The node's calls a save makes, given by the caller so the decision is
 * tested without a node. */
export interface SaveCalls {
  write: (expect: WriteExpect) => Promise<Outcome<FileVersion>>;
  /** The file's first page, as the disk has it now. */
  read: () => Promise<Outcome<FileRead>>;
}

/** A refused write, read back for the banner. */
async function conflictOf(calls: SaveCalls, mine: string): Promise<Saved> {
  const disk = await calls.read();
  return {
    kind: "conflict",
    disk: disk.ok ? disk.value.version : null,
    // Only a file read whole in its first page is compared line by line:
    // a longer one's first page against the whole edit would count all
    // the rest as added.
    diskText:
      disk.ok &&
      disk.value.content.kind === "text" &&
      (disk.value.content.next ?? null) === null
        ? disk.value.content.text
        : null,
    mine,
  };
}

/** Save `text` over the file if it is still `base`, or anyway when `base`
 * is `null` (the person's Overwrite). At most one write is sent. */
export async function saveText(
  calls: SaveCalls,
  text: string,
  base: FileVersion | null,
): Promise<Saved> {
  const expect = saveGuard(base);
  if (expect === null) {
    return {
      kind: "failed",
      message:
        "This file was too large to be compared when it was read, so it cannot be saved without overwriting. Reload it and edit again.",
    };
  }
  const written = await calls.write(expect);
  if (written.ok) {
    return { kind: "saved", version: written.value };
  }
  // Only a guarded write's refusal is a conflict to show; an Overwrite's
  // failure is said as the node said it.
  if (expect.kind === "content" && nodeError(written.problem) === "conflict") {
    return conflictOf(calls, text);
  }
  return { kind: "failed", message: written.problem.message };
}
