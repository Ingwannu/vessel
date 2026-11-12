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
//   until the next page arrives (it may continue there). Editing needs
//   the whole file, so Edit reads the rest first (at most 1 MiB).
// - Cost: a file larger than the write bound can be read, not edited.

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import type { Problem } from "../../data/outcome";
import { readFile, writeFile } from "../../data/workspace/files";
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
  | { kind: "conflict"; disk: FileVersion | null; diskText: string | null }
  | { kind: "failed"; message: string };

export interface OpenFile {
  text: FileText;
  /** Read the next page (the reader neared the end). */
  more: () => void;
  /** Read every page left (to edit), then call `then` with the text. */
  whole: () => Promise<string | null>;
  /** Write `text` if the file is still the version read, or anyway. */
  save: (text: string, force: boolean) => Promise<Saved>;
  /** Read it again from the start. */
  reload: () => void;
}

/** `page` added to what was read. */
function joined(
  before: Extract<FileText, { kind: "text" }> | null,
  version: FileVersion,
  page: string,
  next: number | null,
): Extract<FileText, { kind: "text" }> {
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
      const before = latest.current;
      const read = await readFile(fleet, node, root, path, offset);
      if (!read.ok) {
        return { kind: "failed", problem: read.problem };
      }
      const content = read.value.content;
      switch (content.kind) {
        case "binary":
        case "unknown":
          return { kind: "binary", version: read.value.version };
        case "text":
          return joined(
            offset === 0 || before.kind !== "text" ? null : before,
            read.value.version,
            content.text,
            content.next ?? null,
          );
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

  const whole = useCallback(async (): Promise<string | null> => {
    let now = latest.current;
    while (now.kind === "text" && now.next !== null) {
      now = await page(now.next);
      latest.current = now;
    }
    setText(now);
    return now.kind === "text" ? now.lines.join("\n") : null;
  }, [page]);

  const save = useCallback(
    async (body: string, force: boolean): Promise<Saved> => {
      const now = latest.current;
      const expect: WriteExpect =
        force || now.kind !== "text"
          ? { kind: "any" }
          : { kind: "version", version: now.version };
      const written = await writeFile(fleet, node, root, path, body, expect);
      if (written.ok) {
        const saved = joined(null, written.value, body, null);
        latest.current = saved;
        setText(saved);
        return { kind: "saved" };
      }
      if (nodeError(written.problem) !== "conflict") {
        return { kind: "failed", message: written.problem.message };
      }
      // Both sides: what is on disk now, read fresh, beside the edit.
      const disk = await readFile(fleet, node, root, path, 0);
      return {
        kind: "conflict",
        disk: disk.ok ? disk.value.version : null,
        diskText:
          disk.ok && disk.value.content.kind === "text"
            ? disk.value.content.text
            : null,
      };
    },
    [fleet, node, root, path],
  );

  const reload = useCallback(() => {
    setText({ kind: "loading" });
    setRound((value) => value + 1);
  }, []);

  return { text, more, whole, save, reload };
}
