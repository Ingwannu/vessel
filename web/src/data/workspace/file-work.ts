// The page's background file work on a node (project icons, the Files
// tab's Modified and Size facts) in one line per node, a few at a time.
//
// [Decision Log]
// - Purpose: background asks must not be refused, nor crowd out what a
//   person does.
// - Constraint: the node runs file work (`file.inspect`, `project.icon`,
//   `file.duplicate`, `file.read_bytes`, `file.suggest`, `folder.suggest`)
//   on four slots and refuses a fifth at once as `busy`, without a queue.
//   A sidebar of five projects, or the Files tab with four folders open in
//   Modified order, asked five at once: one was refused and its icon or
//   its folder's order was silently lost.
// - Alternatives: one line per caller (each still fills the slots beside
//   the other); retrying `busy` (more calls into a full node, and the
//   person's action can still lose); a queue on the node (a protocol
//   change for what the page can pace itself).
// - Choice: one limiter per node shared by every background caller, at
//   most `BACKGROUND_SLOTS` out, the rest waiting in order. Two of the
//   node's four stay free for a person's Duplicate, an image's bytes or a
//   file search while icons and facts are read.
// - Cost: a large sidebar or many open folders fill in two at a time
//   (each call is milliseconds on the node); another client of the same
//   node (a second tab, the native app) is not counted.

import type { NodeKey } from "../../wire/bridge";

/** Background file work out at once per node: below the node's four file
 * work slots, so a person's own file action keeps a slot. */
export const BACKGROUND_SLOTS = 2;

interface Line {
  out: number;
  waiting: (() => void)[];
}

const lines = new Map<NodeKey, Line>();

function lineOf(node: NodeKey): Line {
  const known = lines.get(node);
  if (known !== undefined) return known;
  const line: Line = { out: 0, waiting: [] };
  lines.set(node, line);
  return line;
}

/** A slot on the line: taken at once when one is free, else handed over
 * by the work before it as that ends. */
function take(line: Line): Promise<void> {
  if (line.out < BACKGROUND_SLOTS) {
    line.out += 1;
    return Promise.resolve();
  }
  return new Promise((go) => {
    line.waiting.push(go);
  });
}

function give(line: Line): void {
  const next = line.waiting.shift();
  if (next === undefined) {
    line.out -= 1;
  } else {
    next();
  }
}

/** `work` run on `node`'s background line: waits for a slot, and frees it
 * however the work ends. */
export async function inBackground<T>(
  node: NodeKey,
  work: () => Promise<T>,
): Promise<T> {
  const line = lineOf(node);
  await take(line);
  try {
    return await work();
  } finally {
    give(line);
  }
}
