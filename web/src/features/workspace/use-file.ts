// One file of the Files tab: read a page at a time as the reader goes
// (the node pages 256 KiB, `next` until the end), its lines kept with the
// version it was read at; saved with that version as the guard, so a
// change made on disk since (an agent, another editor) is a `conflict`
// the page shows, never overwritten unasked.
//
// [Decision Log]
// - Purpose: a viewer for any file, an editor for text up to the node's
//   write bound (1 MiB), and agent-orchestrator's rule that a save which
//   finds the file changed is refused, with GOAT's choices then (keep my
//   edits and reload, or overwrite).
// - Constraint: `file.write` is guarded by the content's SHA-256
//   (`data/workspace/save.ts`); a read's version holds the size, the time
//   in milliseconds (shown only) and that hash (for files up to 8 MiB). A
//   page is cut at a character boundary, so pages join without breaking a
//   character.
// - Choice: lines split once per page; the last line of a page is held
//   until the next page arrives (it may continue there), and a page of
//   another version than the pages before it (the file changed between
//   two reads) starts the reading again, so lines of two versions are
//   never joined. Editing needs the whole file, so Edit reads the rest
//   first (at most 1 MiB), and the edit keeps the version it started
//   from: that, not whatever was read since, is the save's guard.
// - Cost: a file larger than the write bound can be read, not edited.
// - The disk (ADR 0105): while the file is shown, `use-disk.ts` follows
//   it on a node with the file panels' calls; `disk` turns `changed` when
//   it moved past what is shown, and `refresh` reads it again without
//   blanking the view.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { Problem } from "../../data/outcome";
import { readFile, writeFile } from "../../data/workspace/files";
import { type Saved, saveText } from "../../data/workspace/save";
import type { NodeKey } from "../../wire/bridge";
import type { FileVersion, RelPath, WorkspaceRoot } from "../../wire/proto";
import { type Disk, useDisk } from "./use-disk";

/** The most `file.write` takes, in bytes. */
export const WRITE_LIMIT = 1024 * 1024;

/** What is known of the file. */
export type FileText =
  | { kind: "loading" }
  | { kind: "binary"; version: FileVersion }
  | { kind: "failed"; problem: Problem }
  | {
      kind: "text";
      version: FileVersion;
      lines: readonly string[];
      /** Where the next page starts, while there is one. */
      next: number | null;
      /** The end of the last page read, held until the next. */
      tail: string;
    };

/** The whole text of a file and the version it was read at. */
export interface Whole {
  text: string;
  version: FileVersion;
}

export interface OpenFile {
  text: FileText;
  /** Read the next page (the reader neared the end). */
  more: () => void;
  /** Read every page left (to edit): the text and its version. */
  whole: () => Promise<Whole | null>;
  /** Write `text` if the file is still `base` (the version the edit
   * started from), or anyway (`base` null). */
  save: (text: string, base: FileVersion | null) => Promise<Saved>;
  /** Read it again from the start. */
  reload: () => void;
  /** The disk moved past what is shown (`file.observe`): a view follows
   * it with `refresh` unless an edit is open. */
  disk: Disk;
  /** Read it again from the start, what is shown kept until it arrives. */
  refresh: () => void;
}

type TextRead = Extract<FileText, { kind: "text" }>;

/** Whether two reads are of one version: by the content's hash where both
 * have one, else (a file over 8 MiB) by its size and time. */
function sameVersion(a: FileVersion, b: FileVersion): boolean {
  const hash = a.hash ?? null;
  return hash === null || (b.hash ?? null) === null
    ? a.size === b.size && a.modified === b.modified
    : a.size === b.size && hash === b.hash;
}

/** `page` added to what was read. */
function joined(
  before: TextRead | null,
  version: FileVersion,
  page: string,
  next: number | null,
): TextRead {
  const text = (before?.tail ?? "") + page;
  const parts = text.split("\n");
  const tail = next === null ? "" : (parts.pop() ?? "");
  return {
    kind: "text",
    version,
    lines: [...(before?.lines ?? []), ...parts],
    next,
    tail,
  };
}

export function useFile(
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
): OpenFile {
  const fleet = useFleet();
  const [text, setText] = useState<FileText>({ kind: "loading" });
  const [round, setRound] = useState(0);
  const latest = useRef(text);
  useEffect(() => {
    latest.current = text;
  }, [text]);
  const reading = useRef(false);

  const page = useCallback(
    async (offset: number): Promise<FileText> => {
      const before = offset === 0 ? null : latest.current;
      const read = await readFile(fleet, node, root, path, offset);
      if (!read.ok) {
        return { kind: "failed", problem: read.problem };
      }
      const { content, version } = read.value;
      switch (content.kind) {
        case "binary":
        case "unknown":
          return { kind: "binary", version };
        case "text":
          if (before?.kind !== "text") {
            return joined(null, version, content.text, content.next ?? null);
          }
          if (!sameVersion(before.version, version)) {
            // Changed since the pages before: read it again from its start.
            const first = await readFile(fleet, node, root, path, 0);
            if (!first.ok) {
              return { kind: "failed", problem: first.problem };
            }
            return first.value.content.kind === "text"
              ? joined(
                  null,
                  first.value.version,
                  first.value.content.text,
                  first.value.content.next ?? null,
                )
              : { kind: "binary", version: first.value.version };
          }
          return joined(before, version, content.text, content.next ?? null);
      }
    },
    [fleet, node, root, path],
  );

  useEffect(() => {
    let current = true;
    reading.current = true;
    void page(0).then((first) => {
      reading.current = false;
      if (current) {
        latest.current = first;
        setText(first);
      }
    });
    return () => {
      current = false;
    };
  }, [page, round]);

  const more = useCallback(() => {
    const now = latest.current;
    if (reading.current || now.kind !== "text" || now.next === null) {
      return;
    }
    reading.current = true;
    void page(now.next).then((next) => {
      reading.current = false;
      // Read past meanwhile (Edit read the rest, or a reload began): this
      // page would put a part of the file back over the whole.
      if (latest.current !== now) {
        return;
      }
      latest.current = next;
      setText(next);
    });
  }, [page]);

  const whole = useCallback(async (): Promise<Whole | null> => {
    let now = latest.current;
    while (now.kind === "text" && now.next !== null) {
      now = await page(now.next);
      latest.current = now;
    }
    setText(now);
    return now.kind === "text"
      ? { text: now.lines.join("\n"), version: now.version }
      : null;
  }, [page]);

  const write = useCallback(
    async (body: string, base: FileVersion | null): Promise<Saved> => {
      const saved = await saveText(
        {
          write: (expect) => writeFile(fleet, node, root, path, body, expect),
          read: () => readFile(fleet, node, root, path, 0),
        },
        body,
        base,
      );
      if (saved.kind === "saved") {
        const read = joined(null, saved.version, body, null);
        latest.current = read;
        setText(read);
      }
      return saved;
    },
    [fleet, node, root, path],
  );

  // A save in flight: the disk's watch judges an answer only once the
  // save has set what is shown.
  const writing = useRef<Promise<Saved> | null>(null);
  const save = useCallback(
    async (body: string, base: FileVersion | null): Promise<Saved> => {
      const saving = write(body, base);
      writing.current = saving;
      try {
        return await saving;
      } finally {
        if (writing.current === saving) {
          writing.current = null;
        }
      }
    },
    [write],
  );

  const shown = useCallback(() => {
    const now = latest.current;
    return now.kind === "text" || now.kind === "binary"
      ? (now.version.hash ?? null)
      : null;
  }, []);
  const inFlight = useCallback(() => writing.current, []);
  const { disk, settled } = useDisk(node, root, path, shown, inFlight);

  const reload = useCallback(() => {
    settled();
    setText({ kind: "loading" });
    setRound((value) => value + 1);
  }, [settled]);

  const refresh = useCallback(() => {
    reading.current = true;
    void page(0).then((first) => {
      reading.current = false;
      latest.current = first;
      setText(first);
      settled();
    });
  }, [page, settled]);

  return { text, more, whole, save, reload, disk, refresh };
}
