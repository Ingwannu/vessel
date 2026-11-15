import { describe, expect, it } from "vitest";

import { findAll } from "./find";
import { outline } from "./outline";
import { rowIndex, rowsOf } from "./rows";
import type { Entry } from "./timeline-model";
import { toolRuns } from "./tool-runs";

function prompt(seq: number, text: string): Entry {
  return {
    kind: "prompt",
    key: `u${String(seq)}`,
    seq,
    at: "",
    turn: 1,
    text,
    replayed: false,
    attachments: [],
  };
}

function message(seq: number, text: string): Entry {
  return { kind: "message", key: `m${String(seq)}`, seq, at: "", text };
}

function thought(seq: number): Entry {
  return { kind: "thought", key: `h${String(seq)}`, seq, at: "", text: "…" };
}

function tool(seq: number): Entry {
  return {
    kind: "tool",
    key: `t${String(seq)}`,
    seq,
    at: "",
    tool: {
      id: `c${String(seq)}`,
      title: "Read",
      kind: "read",
      status: "completed",
      content: [],
      locations: [],
      input: null,
      output: null,
    },
  };
}

/** A conversation as `fake-acp`'s `entries` makes it: a message, a
 * thought and a tool call in turn, after one prompt. */
function conversation(count: number): Entry[] {
  const entries: Entry[] = [prompt(0, "go")];
  for (let index = 0; index < count; index += 1) {
    const seq = index + 1;
    switch (index % 3) {
      case 0:
        entries.push(
          message(
            seq,
            `Entry ${String(index)} names module_${String(index)}.rs`,
          ),
        );
        break;
      case 1:
        entries.push(thought(seq));
        break;
      default:
        entries.push(tool(seq));
    }
  }
  return entries;
}

describe("the window's rows", () => {
  it("are the entries themselves when nothing is folded", () => {
    const entries = conversation(30);
    expect(rowsOf(entries, null, new Set())).toBe(entries);
    expect(rowIndex(entries, "m10", null)).toBe(10);
    expect(rowIndex(entries, "nowhere", null)).toBe(-1);
  });

  it("leave out a closed run's members but its lead, and keep an open run's", () => {
    const entries = conversation(9);
    const folds = toolRuns(entries);
    // u0, m1, [h2 t3], m4, [h5 t6], m7, [h8 t9]
    const closed = rowsOf(entries, folds, new Set());
    expect(closed.map((row) => row.key)).toEqual([
      "u0",
      "m1",
      "h2",
      "m4",
      "h5",
      "m7",
      "h8",
    ]);
    const opened = rowsOf(entries, folds, new Set(["h5"]));
    expect(opened.map((row) => row.key)).toEqual([
      "u0",
      "m1",
      "h2",
      "m4",
      "h5",
      "t6",
      "m7",
      "h8",
    ]);
    // A folded member is shown by its run's lead.
    expect(rowIndex(closed, "t6", folds)).toBe(4);
    expect(rowIndex(opened, "t6", folds)).toBe(5);
    expect(rowIndex(closed, "t99", folds)).toBe(-1);
  });

  // O(n²) by design: its scale is the point, and the 5 s default fails
  // under build-machine load (1-2 s when idle), so it gets 30 s.
  it("hold every key find and the outline hand over, folded or not", () => {
    // Find looks in prompts and messages, the outline lists prompts: none
    // is ever inside a run, so each is a row of its own either way, and
    // the index it names is the row that draws it.
    const entries = conversation(20_000);
    const folds = toolRuns(entries);
    for (const rows of [
      rowsOf(entries, null, new Set()),
      rowsOf(entries, folds, new Set()),
    ]) {
      const keys = [
        ...findAll(entries, "names module").map((match) => match.key),
        ...outline(entries).map((item) => item.key),
      ];
      expect(keys).toHaveLength(6_668);
      for (const key of keys) {
        const index = rowIndex(rows, key, folds);
        expect(rows[index]?.key).toBe(key);
      }
    }
    // Folded, a third of the conversation's 20,001 entries are rows.
    expect(rowsOf(entries, folds, new Set())).toHaveLength(13_335);
  }, 30_000);
});
