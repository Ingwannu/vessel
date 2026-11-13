import { describe, expect, test } from "vitest";

import {
  countSpoken,
  countText,
  findAll,
  findInSlices,
  fold,
  occurrences,
  queryOf,
  readableText,
  spansOf,
  step,
} from "./find";
import type { Entry } from "./timeline-model";

function prompt(seq: number, text: string): Entry {
  return {
    kind: "prompt",
    key: `p${String(seq)}`,
    seq,
    at: "",
    text,
    replayed: false,
    attachments: [],
    turn: seq,
  };
}

function message(seq: number, text: string): Entry {
  return { kind: "message", key: `m${String(seq)}`, seq, at: "", text };
}

describe("folding", () => {
  test("case, accents and white space fold as agent-orchestrator's do", () => {
    expect(fold("Café\n  CRÈME")).toBe("cafe creme");
    expect(queryOf("  Hello   World ")).toBe("hello world");
  });

  test("Markdown's marks are not text", () => {
    expect(readableText("**done** and `npm test`")).toBe("done and npm test");
    expect(readableText("see [the docs](https://x.test/a)")).toBe(
      "see the docs",
    );
    expect(readableText("# Title\n- one\n> quoted")).toBe("Title\none\nquoted");
    // An underscore inside a word is the word's; at its edges, emphasis.
    expect(readableText("names `src/module_300.rs` and _this_")).toBe(
      "names src/module_300.rs and this",
    );
    expect(readableText("__init__ and snake_case_name")).toBe(
      "init and snake_case_name",
    );
  });

  test("occurrences do not overlap", () => {
    expect(occurrences("aaaa", "aa")).toBe(2);
    expect(occurrences("abc", "")).toBe(0);
  });
});

describe("finding", () => {
  const entries: Entry[] = [
    prompt(1, "Fix the build"),
    message(2, "The **build** failed; I fixed the build script."),
    { kind: "thought", key: "t3", seq: 3, at: "", text: "build build" },
    {
      kind: "log",
      key: "l4",
      seq: 4,
      at: "",
      line: "build",
    },
  ];

  test("prompts and messages, every occurrence, in order", () => {
    expect(findAll(entries, "BUILD")).toEqual([
      { key: "p1", nth: 0 },
      { key: "m2", nth: 0 },
      { key: "m2", nth: 1 },
    ]);
  });

  test("an empty query finds nothing", () => {
    expect(findAll(entries, "   ")).toEqual([]);
  });

  test("in slices, the same answer, and none once cancelled", async () => {
    const many = Array.from({ length: 1000 }, (_, index) =>
      index % 2 === 0 ? prompt(index, "needle") : message(index, "hay"),
    );
    const noPause = () => Promise.resolve();
    const found = await findInSlices(many, "needle", () => false, noPause);
    expect(found).toHaveLength(500);
    let slices = 0;
    const gone = await findInSlices(
      many,
      "needle",
      () => slices > 0,
      () => {
        slices += 1;
        return Promise.resolve();
      },
    );
    expect(gone).toBeNull();
  });
});

describe("moving between matches", () => {
  test("next and previous wrap, and say so", () => {
    expect(step(3, 1, 1)).toEqual({ index: 2, wrapped: false });
    expect(step(3, 2, 1)).toEqual({ index: 0, wrapped: true });
    expect(step(3, 0, -1)).toEqual({ index: 2, wrapped: true });
    expect(step(0, 0, 1)).toEqual({ index: 0, wrapped: false });
  });

  test("the count as written and as spoken", () => {
    expect(countText(12, 2)).toBe("3/12");
    expect(countSpoken(12, 2)).toBe("3 of 12");
    expect(countText(0, 0)).toBe("No results");
  });
});

describe("where a match is in the drawn text", () => {
  test("folded as the count folds, mapped back to the text", () => {
    const text = "A Café\n\n  and  CAFE au lait";
    const spans = spansOf(text, "cafe");
    expect(spans.map(({ start, end }) => text.slice(start, end))).toEqual([
      "Café",
      "CAFE",
    ]);
    // Runs of white space are one space, as in the query.
    const runs = spansOf(text, "café and");
    expect(runs.map(({ start, end }) => text.slice(start, end))).toEqual([
      "Café\n\n  and",
    ]);
    // Characters outside the basic plane keep their two code units.
    const wide = "go 🚀 now, 🚀 again";
    expect(
      spansOf(wide, "🚀 again").map(({ start, end }) => wide.slice(start, end)),
    ).toEqual(["🚀 again"]);
    expect(spansOf(text, "  ")).toEqual([]);
  });

  test("an accent written apart is the letter's", () => {
    const text = "cafe\u0301 noir";
    const spans = spansOf(text, "café");
    expect(spans.map(({ start, end }) => text.slice(start, end))).toEqual([
      "cafe\u0301",
    ]);
  });
});
