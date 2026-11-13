// A save refused because the file changed on disk since it was read
// (the content's hash read was the guard, `data/workspace/save.ts`): both sides' state, then GOAT's two
// ways on: reload from disk (the edits are dropped) or overwrite (the
// disk's change is). Nothing is written until one is chosen.

import { useMemo } from "react";

import { when } from "../../data/format";
import { compareLists } from "../../data/workspace/command-changes";
import { sizeWord } from "../../data/workspace/size";
import { Banner, Button } from "../../ui/kit";
import type { FileVersion } from "../../wire/proto";

/** A version's modification time (milliseconds since the epoch; 0 when
 * the machine's file system does not say) as the page writes a time. */
function modified(version: FileVersion): string {
  return version.modified === 0
    ? "at a time the machine does not say"
    : `modified ${when(new Date(version.modified).toISOString())}`;
}

/** How many lines differ between the disk's text and the edit. */
function difference(disk: string, mine: string): string {
  const lines = compareLists(disk.split("\n"), mine.split("\n"));
  const added = lines.filter((line) => line.change === "added").length;
  const removed = lines.filter((line) => line.change === "removed").length;
  return `your edit adds ${String(added)} lines and removes ${String(removed)}`;
}

/** The most lines compared in the page (both sides are at most 1 MiB,
 * but a comparison is quadratic in lines). */
const COMPARED = 2000;

export function FileConflict({
  mine,
  read,
  disk,
  diskText,
  busy,
  onReload,
  onOverwrite,
}: {
  /** The edit that was refused (not the one typed since: the
   * comparison is worked out once, not on every key). */
  mine: string;
  /** The version the edit started from. */
  read: FileVersion;
  /** The version on disk now, when it could be read. */
  disk: FileVersion | null;
  /** The disk's text, when it is text (its first page). */
  diskText: string | null;
  busy: boolean;
  onReload: () => void;
  onOverwrite: () => void;
}) {
  const compared = useMemo(
    () =>
      diskText === null ||
      diskText.split("\n").length > COMPARED ||
      mine.split("\n").length > COMPARED
        ? null
        : difference(diskText, mine),
    [diskText, mine],
  );
  return (
    <div className="file-strip">
      <Banner tone="error">
        <strong>File changed on disk.</strong> Your unsaved edits are kept;
        nothing was written.
        <ul className="file-sides">
          <li>
            Read: {sizeWord(read.size)}, {modified(read)}
          </li>
          <li>
            {disk === null
              ? "On disk now: the file is gone or cannot be read; Overwrite writes it again."
              : `On disk now: ${sizeWord(disk.size)}, ${modified(disk)}`}
          </li>
          {compared === null ? null : <li>Against the disk, {compared}.</li>}
        </ul>
        <span className="file-strip-actions">
          <Button size="xs" onClick={onReload} disabled={busy}>
            Reload from disk
          </Button>
          <Button size="xs" variant="danger" onClick={onOverwrite} busy={busy}>
            Overwrite
          </Button>
        </span>
      </Banner>
    </div>
  );
}
