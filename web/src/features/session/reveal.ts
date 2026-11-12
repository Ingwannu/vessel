// The timeline's one seam for find and the outline: bring an entry into
// view by its key and mark a match in it (DESIGN.md §6.16).
//
// [Decision Log]
// - Purpose: find and the outline decide what to show from the model
//   (`data/find.ts`, `data/outline.ts`); something must then put that
//   entry on screen. The timeline is windowed (track P, use-window.ts):
//   the entry may not be drawn, so the callers cannot reach into its DOM,
//   and the timeline must not learn what a find is.
// - Choice: the timeline takes a `Reveal` (an entry's key, and for a find
//   the query and which occurrence), and its window goes to that entry's
//   row (use-window.ts's `Target`): it scrolls to where the row is
//   estimated to be, so the row is drawn; once it is, `showReveal` marks
//   the match or gives the entry focus, and says what the window then
//   holds in place while measured heights replace the estimates.
// - The marks are the CSS Custom Highlight API (agent-orchestrator's and
//   GOAT's find): ranges over the drawn text, no element added, nothing
//   for React to reconcile. Only the revealed entry is walked, so the cost
//   is one entry's text. Where the API is missing, the entry is still
//   brought into view.
// - Focus: a find keeps it in its field, as agent-orchestrator's does
//   (Enter goes on to the next match); the outline's jump gives it to the
//   prompt (focusable from script only, timeline-item.tsx), so the
//   keyboard goes on from there.

/** What to bring into view. `seq` changes with every request, so asking
 * for the same match again scrolls to it again. */
export interface Reveal {
  key: string;
  seq: number;
  /** For a find: the query and which of its occurrences in the entry is
   * the current one; `null` for the outline. */
  match: { query: string; nth: number } | null;
}

const ALL = "vessel-find";
const ACTIVE = "vessel-find-active";

/** The browser's highlight registry, where it has one. */
function registry(): HighlightRegistry | null {
  return typeof CSS !== "undefined" && "highlights" in CSS
    ? CSS.highlights
    : null;
}

/** `reveal` when it is a find's, with a match to mark; else `null`. */
export function finding(reveal: Reveal | null): Reveal | null {
  return (reveal?.match ?? null) === null ? null : reveal;
}

/** Remove every find mark. */
export function clearMarks(): void {
  const highlights = registry();
  highlights?.delete(ALL);
  highlights?.delete(ACTIVE);
}

/** The query as a pattern: its words, escaped, with any run of white
 * space between them (Paseo's), ignoring case. */
function pattern(query: string): RegExp | null {
  const words = query
    .trim()
    .split(/\s+/)
    .filter((word) => word !== "");
  if (words.length === 0) {
    return null;
  }
  const escaped = words.map((word) =>
    word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
  );
  return new RegExp(escaped.join("\\s+"), "giu");
}

/** The text nodes a reader sees in `element`, with where each starts in
 * their joined text. Buttons, icons and hidden parts are not text. */
function textNodes(element: Element): { node: Text; at: number }[] {
  const nodes: { node: Text; at: number }[] = [];
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  let at = 0;
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    if (!(node instanceof Text)) {
      continue;
    }
    const parent = node.parentElement;
    if (parent?.closest("button, svg, [aria-hidden='true']") !== null) {
      continue;
    }
    nodes.push({ node, at });
    at += node.data.length;
  }
  return nodes;
}

/** The node and offset of `index` in the joined text. */
function point(
  nodes: readonly { node: Text; at: number }[],
  index: number,
): { node: Text; offset: number } | null {
  for (let at = nodes.length - 1; at >= 0; at -= 1) {
    const entry = nodes[at];
    if (entry !== undefined && entry.at <= index) {
      return { node: entry.node, offset: index - entry.at };
    }
  }
  return null;
}

/** Mark every occurrence of `query` in `element`, the `nth` (or the last,
 * when the drawn text has fewer) as the current one; its range. */
function mark(element: Element, query: string, nth: number): Range | null {
  const highlights = registry();
  const regex = pattern(query);
  if (highlights === null || regex === null) {
    return null;
  }
  const nodes = textNodes(element);
  const text = nodes.map((entry) => entry.node.data).join("");
  const ranges: Range[] = [];
  for (const found of text.matchAll(regex)) {
    const start = point(nodes, found.index);
    const end = point(nodes, found.index + found[0].length);
    if (start === null || end === null) {
      continue;
    }
    const range = document.createRange();
    range.setStart(start.node, start.offset);
    range.setEnd(end.node, end.offset);
    ranges.push(range);
  }
  const current = ranges[Math.min(nth, ranges.length - 1)] ?? null;
  highlights.set(
    ALL,
    new Highlight(...ranges.filter((range) => range !== current)),
  );
  if (current === null) {
    highlights.delete(ACTIVE);
  } else {
    highlights.set(ACTIVE, new Highlight(current));
  }
  return current;
}

/** Show `reveal` in its drawn `row`: the match marked, or the entry
 * given focus; what the window holds in place, the match's range or the
 * entry. Scrolls nothing: the window does (use-window.ts). */
export function showReveal(row: Element, reveal: Reveal): Element | Range {
  const entry =
    row.querySelector(`[data-entry="${CSS.escape(reveal.key)}"]`) ?? row;
  if (reveal.match === null) {
    clearMarks();
    if (entry instanceof HTMLElement) {
      entry.focus({ preventScroll: true });
    }
    return entry;
  }
  return mark(entry, reveal.match.query, reveal.match.nth) ?? entry;
}
