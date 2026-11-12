import { describe, expect, it } from "vitest";

import type { FileDiff, Hunk } from "../../wire/proto";
import { addPage, countLines, diffRows, hunkHeader } from "./diff-rows";

const HUNK: Hunk = {
  old_start: 10,
  old_lines: 3,
  new_start: 10,
  new_lines: 4,
  section: "fn main()",
  lines: [
    { kind: "context", text: "let a = 1;" },
    { kind: "removed", text: "let b = 2;" },
    { kind: "added", text: "let b = 3;" },
    { kind: "added", text: "let c = 4;" },
    { kind: "context", text: "}" },
    { kind: "no_newline", text: "\\ No newline at end of file" },
  ],
};

function page(hunks: Hunk[], moreFrom: number | null): FileDiff {
  return {
    path: "src/main.rs",
    status: "modified",
    content: { kind: "hunks", hunks },
    more_from: moreFrom,
  };
}

describe("a diff as rows", () => {
  it("writes the hunk's header as git does", () => {
    expect(hunkHeader(HUNK)).toBe("@@ -10,3 +10,4 @@");
  });

  it("numbers each side as it goes", () => {
    const rows = diffRows([HUNK], 0, "f:");
    const numbers = rows.flatMap((row) =>
      row.kind === "line" ? [[row.old, row.new]] : [],
    );
    expect(numbers).toEqual([
      [10, 10],
      [11, null],
      [null, 11],
      [null, 12],
      [12, 13],
      [null, null],
    ]);
    expect(rows[0]).toMatchObject({ kind: "hunk", section: "fn main()" });
  });

  it("says when the node sent only the start of a hunk", () => {
    expect(diffRows([HUNK], 0, "f:").some((row) => row.kind === "cut")).toBe(
      false,
    );
    const long: Hunk = { ...HUNK, old_lines: 9000, new_lines: 9001 };
    expect(diffRows([long], 0, "f:").at(-1)).toEqual({
      kind: "cut",
      key: "f:h0:cut",
    });
  });

  it("keeps keys stable as pages are added", () => {
    const first = addPage(null, page([HUNK], 1), "f:");
    const second = addPage(first, page([HUNK], null), "f:");
    if (second.kind !== "rows") {
      throw new Error("rows expected");
    }
    const keys = second.rows.map((row) => row.key);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys).toContain("f:h1");
    expect(second.more).toBeNull();
  });

  it("says what a binary or oversized file is instead of rows", () => {
    expect(
      addPage(null, { ...page([], null), content: { kind: "binary" } }, "f:"),
    ).toEqual({ kind: "binary" });
    expect(
      addPage(
        null,
        { ...page([], null), content: { kind: "too_large", bytes: 9 } },
        "f:",
      ),
    ).toEqual({ kind: "too_large", bytes: 9 });
  });

  it("counts what a page adds and removes", () => {
    expect(countLines(diffRows([HUNK], 0, ""))).toEqual({
      added: 2,
      removed: 1,
    });
  });
});
