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
// - Constraint: `file.write` takes `expect: version`; the version holds
//   the size, the modification time in nanoseconds and the content's
//   SHA-256 (for files up to 8 MiB). A page is cut at a character
//   boundary, so pages join without breaking a character.
// - Choice: lines split once per page; the last line of a page is held
//   until the next page arrives (it may continue there), and a page of
//   another version than the pages before it (the file changed between
//   two reads) starts the reading again, so lines of two versions are
//   never joined. Editing needs the whole file, so Edit reads the rest
//   first (at most 1 MiB), and the edit keeps the version it started
//   from: that, not whatever was read since, is the save's guard.
// - Cost: a file larger than the write bound can be read, not edited.
//
// [Decision Log: a save the node refuses although nothing changed]
// - Problem: the node compares the version's modification time exactly,
//   in nanoseconds since 1970 (about 1.7e18), and the page holds it as a
//   JavaScript number (exact only to 2^53): the bridge's JSON rounds it to
//   a multiple of 256, so on a file system with nanosecond times the
//   guarded write of an untouched file is refused as `conflict` nearly
//   every time. The wire cannot carry it exactly to the page.
// - Alternatives: parse the bridge's frames with the source text kept
//   and write the number back raw (`JSON.rawJSON`; the bridge and every
//   browser would need it); show the conflict and have the person press
//   Overwrite (teaching them to); the node to compare size and hash when
//   the version has a hash, or the time as text (a wire change; reported
//   as this track's wire gap).
// - Choice: on `conflict`, read the file's version; when its size and
//   content hash are the ones the edit started from (the disk holds what
//   the edit began with, so a write loses nothing of anyone's), write it
//   with `any`. Anything else is the conflict shown.
// - Cost: a change landing between that read and the write (one round
//   trip) would be overwritten; the node's comparison by hash removes the
//   window and this fallback with it.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { Problem } from "../../data/outcome";
import { readFile, sameContent, writeFile } from "../../data/workspace/files";
import { nodeError } from "../../data/workspace/root";
import type { NodeKey } from "../../wire/bridge";
import type {
  FileVersion,
  RelPath,
  WorkspaceRoot,
  WriteExpect,
} from "../../wire/proto";

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

/** A save's answer. */
export type Saved =
  | { kind: "saved" }
  | {
      kind: "conflict";
      disk: FileVersion | null;
      diskText: string | null;
      /** The text that was refused. */
      mine: string;
    }
  | { kind: "failed"; message: string };

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
}

type TextRead = Extract<FileText, { kind: "text" }>;

/** Whether two reads are of one version. */
function sameVersion(a: FileVersion, b: FileVersion): boolean {
  return (
    a.size === b.size &&
    a.modified === b.modified &&
    (a.hash ?? null) === (b.hash ?? null)
  );
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

  const save = useCallback(
    async (body: string, base: FileVersion | null): Promise<Saved> => {
      const hash = base?.hash ?? null;
      const expect: WriteExpect =
        hash === null ? { kind: "any" } : { kind: "content", hash };
      let written = await writeFile(fleet, node, root, path, body, expect);
      if (
        !written.ok &&
        base !== null &&
        nodeError(written.problem) === "conflict"
      ) {
        // Both sides: what is on disk now, read fresh, beside the edit;
        // or, when it is what the edit started from, the write again
        // (the Decision Log above).
        const disk = await readFile(fleet, node, root, path, 0);
        if (!disk.ok || !sameContent(base, disk.value.version)) {
          return {
            kind: "conflict",
            disk: disk.ok ? disk.value.version : null,
            diskText:
              disk.ok && disk.value.content.kind === "text"
                ? disk.value.content.text
                : null,
            mine: body,
          };
        }
        written = await writeFile(fleet, node, root, path, body, {
          kind: "any",
        });
      }
      if (!written.ok) {
        return { kind: "failed", message: written.problem.message };
      }
      const saved = joined(null, written.value, body, null);
      latest.current = saved;
      setText(saved);
      return { kind: "saved" };
    },
    [fleet, node, root, path],
  );

  const reload = useCallback(() => {
    setText({ kind: "loading" });
    setRound((value) => value + 1);
  }, []);

  return { text, more, whole, save, reload };
}
