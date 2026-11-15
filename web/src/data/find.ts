// Find in a session (DESIGN.md §6.16): what matches a query in the
// conversation's model (`Timeline.entries`), never in the document.
//
// [Decision Log]
// - Purpose: all three references find text in a session (Mod+F); the
//   timeline is being windowed (milestone 11, track P), so text that is
//   not drawn must still be found, and the match must be reachable.
// - Constraint: a session can hold thousands of entries; a search on every
//   keystroke must not hold the page. The model is replaced per batch of
//   records, but an entry that did not change keeps its object.
// - Alternatives: the browser's own find (misses what is not drawn); the
//   DOM (the same, and slower); a server-side search (the references with
//   history beyond the loaded window have one; Vessel's page holds the
//   whole log, so the model is the whole conversation).
// - Choice: the prompts and the agent's messages are searched, as
//   agent-orchestrator, GOAT and Paseo search messages and not tool
//   output. Matching folds case, accents and runs of white space
//   (agent-orchestrator's `foldFindText`), over the text as it reads:
//   Markdown's marks are removed before folding (`readableText`), so
//   `**done**` is found by "done" and a link by its words. Each entry's
//   folded text is kept in a WeakMap by entry object, so a second search
//   folds only what changed. The search runs in slices of `SLICE`
//   entries, yielding between them, and stops when a newer query starts.
// - Cost: the count is of matches in the readable text, which can differ
//   from the drawn text where Markdown renders something else (a table's
//   pipes); the highlight looks for the same folded text in the revealed
//   entry only, and marks what it finds.

import type { Entry } from "./timeline-model";

/** One occurrence: the entry it is in and its index there. */
export interface Match {
  key: string;
  /** Which occurrence within the entry, from 0. */
  nth: number;
}

/** The longest query searched (agent-orchestrator's limit). */
export const MAX_QUERY = 512;

/** Entries searched between two yields. */
const SLICE = 200;

/** The kinds of entry a find looks in. */
export function searchable(entry: Entry): string | null {
  switch (entry.kind) {
    case "command":
    case "prompt":
    case "message":
    case "steer":
      return entry.text;
    case "question":
      return entry.question.message;
    case "ready":
    case "thought":
    case "tool":
    case "plan":
    case "permission":
    case "turn_ended":
    case "turn_failed":
    case "notification":
    case "compaction":
    case "plugin":
    case "error":
    case "log":
    case "status":
    case "profile":
    case "model":
      return null;
  }
}

/** Markdown's marks removed: fences, emphasis, code ticks, headings,
 * quotes and list marks, links as their words. Approximate on purpose:
 * it decides what a reader would call the text, not how it is drawn. An
 * underscore inside a word (`module_300`, `snake_case`) is the word's,
 * as CommonMark reads it, and is kept: only one at a word's edge marks
 * emphasis. */
export function readableText(markdown: string): string {
  return markdown
    .replace(/^\s{0,3}(```|~~~).*$/gm, "")
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/^\s{0,3}(#{1,6}\s+|>\s?|[-*+]\s+|\d+[.)]\s+)/gm, "")
    .replace(/(\*\*|\*|~~|`)/g, "")
    .replace(/(?<![\p{L}\p{N}])_+|_+(?![\p{L}\p{N}])/gu, "");
}

/** `text` folded for matching: decomposed, accents dropped, lower case,
 * white space runs as one space. */
export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/\s+/g, " ");
}

const folded = new WeakMap<Entry, string | null>();

/** An entry's folded readable text, folded once per entry object. */
function foldedOf(entry: Entry): string | null {
  const known = folded.get(entry);
  if (known !== undefined) {
    return known;
  }
  const text = searchable(entry);
  const value =
    text === null
      ? null
      : fold(entry.kind === "message" ? readableText(text) : text);
  folded.set(entry, value);
  return value;
}

/** One character of `text` as `fold` reads it: decomposed, accents
 * dropped, lower case; white space as one space. */
function foldOne(character: string): string {
  return /\s/.test(character)
    ? " "
    : character.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Where `query` occurs in `text` as find matches it (folded as `fold`
 * folds, so the highlight and the count agree: "cafe" marks "café"), as
 * offsets into `text` itself, not overlapping, in order. The timeline
 * marks these in the drawn text of the entry it reveals (reveal.ts). */
export function spansOf(
  text: string,
  query: string,
): { start: number; end: number }[] {
  const needle = queryOf(query);
  if (needle === "") {
    return [];
  }
  // The folded text, and for each of its code units where the character
  // it came from starts and ends in `text`.
  let folded = "";
  const starts: number[] = [];
  const ends: number[] = [];
  let at = 0;
  for (const character of text) {
    const next = at + character.length;
    const one = foldOne(character);
    if (one === "" && ends.length > 0) {
      // A mark written apart (an accent after its letter) belongs to the
      // letter before it: a match ending there takes it in.
      ends[ends.length - 1] = next;
    } else if (!(one === " " && folded.endsWith(" "))) {
      starts.push(...new Array<number>(one.length).fill(at));
      ends.push(...new Array<number>(one.length).fill(next));
      folded += one;
    }
    at = next;
  }
  const spans: { start: number; end: number }[] = [];
  for (
    let found = folded.indexOf(needle);
    found !== -1;
    found = folded.indexOf(needle, found + needle.length)
  ) {
    const start = starts[found];
    const end = ends[found + needle.length - 1];
    if (start !== undefined && end !== undefined) {
      spans.push({ start, end });
    }
  }
  return spans;
}

/** How many times `needle` occurs in `haystack`, not overlapping. */
export function occurrences(haystack: string, needle: string): number {
  if (needle === "") {
    return 0;
  }
  let count = 0;
  let at = haystack.indexOf(needle);
  while (at !== -1) {
    count += 1;
    at = haystack.indexOf(needle, at + needle.length);
  }
  return count;
}

/** The query as it is matched: folded and trimmed; `""` searches nothing. */
export function queryOf(text: string): string {
  return fold(text.slice(0, MAX_QUERY)).trim();
}

/** Every match of `query` in `entries`, in order, at once. For tests and
 * short logs; the page uses `findInSlices`. */
export function findAll(entries: readonly Entry[], query: string): Match[] {
  const needle = queryOf(query);
  const matches: Match[] = [];
  if (needle === "") {
    return matches;
  }
  for (const entry of entries) {
    collect(entry, needle, matches);
  }
  return matches;
}

function collect(entry: Entry, needle: string, into: Match[]): void {
  const text = foldedOf(entry);
  if (text === null) {
    return;
  }
  const count = occurrences(text, needle);
  for (let nth = 0; nth < count; nth += 1) {
    into.push({ key: entry.key, nth });
  }
}

/** Gives the page a turn between slices. */
export type Yield = () => Promise<void>;

/** A macrotask: input and paint get their turn before the next slice.
 * A message rather than `setTimeout(0)`: browsers hold a timer set from
 * inside a timer back to at least 4 ms after a few levels, so the 100
 * slices of a 20,000-entry search waited 400 ms between them for nothing
 * (docs/PERFORMANCE.md, "Tracks P and W merged"); a message is
 * delivered as soon as the page has had its turn. */
export const yieldToPage: Yield = () =>
  new Promise((resolve) => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close();
      resolve();
    };
    channel.port2.postMessage(null);
  });

/** Every match of `query` in `entries`, `SLICE` entries at a time,
 * yielding between slices; `null` when `cancelled()` became true
 * meanwhile (a newer query, or the find closed). */
export async function findInSlices(
  entries: readonly Entry[],
  query: string,
  cancelled: () => boolean,
  pause: Yield = yieldToPage,
): Promise<Match[] | null> {
  const needle = queryOf(query);
  const matches: Match[] = [];
  if (needle === "") {
    return matches;
  }
  for (let start = 0; start < entries.length; start += SLICE) {
    if (start > 0) {
      await pause();
      if (cancelled()) {
        return null;
      }
    }
    for (const entry of entries.slice(start, start + SLICE)) {
      collect(entry, needle, matches);
    }
  }
  return matches;
}

/** The match after (`step` 1) or before (`step` -1) `current`, wrapping;
 * and whether it wrapped, which the bar says aloud. */
export function step(
  count: number,
  current: number,
  by: 1 | -1,
): { index: number; wrapped: boolean } {
  if (count === 0) {
    return { index: 0, wrapped: false };
  }
  const next = current + by;
  if (next >= count) {
    return { index: 0, wrapped: true };
  }
  if (next < 0) {
    return { index: count - 1, wrapped: true };
  }
  return { index: next, wrapped: false };
}

/** "3/12", as agent-orchestrator and GOAT write it; "No results". */
export function countText(count: number, index: number): string {
  return count === 0 ? "No results" : `${String(index + 1)}/${String(count)}`;
}

/** "3 of 12", for a screen reader. */
export function countSpoken(count: number, index: number): string {
  return count === 0
    ? "No results"
    : `${String(index + 1)} of ${String(count)}`;
}
