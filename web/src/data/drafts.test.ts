import { expect, test } from "vitest";

import { draftId, fits, MAX_DRAFT_BYTES, wellFormed } from "./drafts";

test("a draft's key as text", () => {
  expect(draftId({ kind: "new" })).toBe("new");
  expect(draftId({ kind: "session", session: "s-1" })).toBe("session:s-1");
});

test("the bound is UTF-8 bytes: Hangul is three each", () => {
  expect(fits("a".repeat(MAX_DRAFT_BYTES))).toBe(true);
  expect(fits("가".repeat(Math.floor(MAX_DRAFT_BYTES / 3)))).toBe(true);
  expect(fits("가".repeat(Math.floor(MAX_DRAFT_BYTES / 3) + 1))).toBe(false);
});

test("a lone surrogate is replaced; a whole emoji is kept", () => {
  const emoji = "😀";
  expect(wellFormed(`a${emoji}b`)).toBe(`a${emoji}b`);
  expect(wellFormed(`a${emoji.slice(0, 1)}b`)).toBe("a�b");
  expect(wellFormed(`${emoji.slice(1)}x`)).toBe("�x");
});
