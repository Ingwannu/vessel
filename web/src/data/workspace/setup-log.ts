// Setup's output as lines a page can show while it runs (the new-session
// page, a worktree's row): the bytes the node pages out
// (`worktree.output`), decoded, with a terminal's colours and cursor
// moves taken out and a carriage return's overwrite applied.
//
// [Decision Log]
// - Purpose: "setup's output is shown as it comes". Setup commands are
//   `npm ci`, `cargo build`, a script: they write progress bars (`\r`)
//   and colours (escape sequences) for a terminal.
// - Constraint: the node keeps 1 MiB per worktree and pages 64 KiB; a
//   page may end inside a UTF-8 character or an escape sequence.
// - Alternatives: the session view's terminal (xterm.js: it draws
//   everything right, but it is the session chunk's 300 KB and a second
//   terminal on a page that shows a box of text); the raw text (escape
//   codes on screen, and every progress step as a line).
// - Choice: a streaming `TextDecoder`, escape sequences dropped by a
//   small scanner (CSI, OSC, and ESC with its intermediates and final, as
//   `tput sgr0`'s `ESC ( B`), `\r` overwriting the line as a terminal
//   would, and erasing the line (`ESC [ K`, `ESC [ 2 K`, which progress
//   bars write after `\r`) applied where the cursor is; the last
//   `MAX_LINES` kept (setup's end is what explains a failure).
// - Cost: colours are lost; a program that draws with cursor moves
//   (a full-screen one) shows its text out of place. Setup commands are
//   not that.

/** How many lines are kept: the end of the output, which is what tells
 * why a setup failed. */
export const MAX_LINES = 2000;

/** The output so far. */
export interface SetupLog {
  /** Finished lines, the oldest dropped past `MAX_LINES`. */
  lines: readonly string[];
  /** The line being written (after the last newline). */
  current: string;
  /** Whether lines were dropped from the start. */
  cut: boolean;
  /** Text held back: an escape sequence cut by the end of a page. */
  pending: string;
}

/** A log with nothing in it. */
export const EMPTY_LOG: SetupLog = {
  lines: [],
  current: "",
  cut: false,
  pending: "",
};

/** Where `plain` leaves an erase of the line, for `overwrite` to apply:
 * from the cursor to the end (`ESC [ K`, `ESC [ 0 K`), or all of it
 * (`ESC [ 2 K`). Unicode noncharacters, which no program's text holds. */
const ERASE_TO_END = "\uFDD0";
const ERASE_LINE = "\uFDD1";

/** `line` after its carriage returns and erases: a terminal starts the
 * line again at each return, writing over what was there, and an erase
 * clears from where the cursor is. */
function overwrite(line: string): string {
  let shown = "";
  for (const part of line.split("\r")) {
    let cursor = 0;
    let run = "";
    const flush = () => {
      shown = shown.slice(0, cursor) + run + shown.slice(cursor + run.length);
      cursor += run.length;
      run = "";
    };
    for (const char of part) {
      if (char === ERASE_TO_END) {
        flush();
        shown = shown.slice(0, cursor);
      } else if (char === ERASE_LINE) {
        flush();
        shown = " ".repeat(cursor);
      } else {
        run += char;
      }
    }
    flush();
  }
  return shown.trimEnd() === "" ? "" : shown;
}

const ESC = 0x1b;
const BEL = 0x07;

/** What an escape sequence from `at` to `end` leaves in the text: an erase
 * of the line as its marker, anything else nothing. */
function kept(text: string, at: number, end: number): string {
  const sequence = text.slice(at, end);
  if (sequence === "\u001b[K" || sequence === "\u001b[0K") {
    return ERASE_TO_END;
  }
  return sequence === "\u001b[2K" ? ERASE_LINE : "";
}

/** Whether `code` is a control character a line of text has no use for
 * (tab, newline and carriage return are kept). */
function dropped(code: number): boolean {
  return (
    (code < 0x20 && code !== 0x09 && code !== 0x0a && code !== 0x0d) ||
    code === 0x7f
  );
}

/** Where the escape sequence at `at` ends (the index after it), or
 * `null` when `text` ends before it does. */
function escapeEnd(text: string, at: number): number | null {
  const kind = text.charCodeAt(at + 1);
  if (Number.isNaN(kind)) {
    return null;
  }
  if (kind === 0x5b) {
    // CSI: parameters and intermediates, then one final byte.
    for (let index = at + 2; index < text.length; index += 1) {
      const code = text.charCodeAt(index);
      if (code >= 0x40 && code <= 0x7e) {
        return index + 1;
      }
      if (code < 0x20 || code > 0x3f) {
        return index;
      }
    }
    return null;
  }
  if (kind === 0x5d) {
    // OSC: up to BEL, or ESC and a backslash.
    for (let index = at + 2; index < text.length; index += 1) {
      const code = text.charCodeAt(index);
      if (code === BEL) {
        return index + 1;
      }
      if (code === ESC) {
        return index + 1 < text.length ? index + 2 : null;
      }
    }
    return null;
  }
  // ESC, any intermediates (0x20-0x2F: `(`, `)`, `#`), then one final
  // byte: `ESC ( B`, `ESC 7`, `ESC =`.
  for (let index = at + 1; index < text.length; index += 1) {
    const code = text.charCodeAt(index);
    if (code < 0x20 || code > 0x2f) {
      return index + 1;
    }
  }
  return null;
}

/** `text` without escape sequences (an erase of the line left as its
 * marker, for `overwrite`) and dropped control characters, and what an
 * unfinished sequence at its end held back. */
export function plain(text: string): { clean: string; pending: string } {
  let clean = "";
  let from = 0;
  let index = 0;
  while (index < text.length) {
    const code = text.charCodeAt(index);
    if (code === ESC) {
      const end = escapeEnd(text, index);
      clean += text.slice(from, index);
      if (end === null) {
        return { clean, pending: text.slice(index) };
      }
      clean += kept(text, index, end);
      index = end;
      from = end;
    } else if (dropped(code)) {
      clean += text.slice(from, index);
      index += 1;
      from = index;
    } else {
      index += 1;
    }
  }
  return { clean: clean + text.slice(from), pending: "" };
}

/** `log` with `text` (decoded output) added. */
export function appendText(log: SetupLog, text: string): SetupLog {
  const { clean, pending } = plain(log.pending + text);
  const pieces = (log.current + clean).split("\n");
  const current = pieces.pop() ?? "";
  if (pieces.length === 0) {
    return { ...log, current, pending };
  }
  const added = pieces.map(overwrite);
  const lines = [...log.lines, ...added];
  const over = lines.length - MAX_LINES;
  return {
    lines: over > 0 ? lines.slice(over) : lines,
    current,
    cut: log.cut || over > 0,
    pending,
  };
}

/** Every line to show, the one being written last (as a terminal shows
 * it: carriage returns applied). */
export function shownLines(log: SetupLog): readonly string[] {
  const current = overwrite(log.current);
  return current === "" ? log.lines : [...log.lines, current];
}

/** A decoder for one worktree's output: UTF-8 across page ends. */
export class OutputDecoder {
  readonly #decoder = new TextDecoder("utf-8");

  /** The text of `bytes`, keeping a character cut by the page's end for
   * the next. */
  text(bytes: Uint8Array): string {
    return this.#decoder.decode(bytes, { stream: true });
  }
}
