import { describe, expect, it } from "vitest";

import { scoreMatch, scorePathMatch } from "./text-match";

// Paseo's `search/text-match.test.ts` (pinned 0.15.4), its exact tiers.
describe("scoreMatch", () => {
  it("returns tier 0 for empty query", () => {
    expect(scoreMatch("", "anything")).toEqual({ tier: 0, offset: 0 });
  });

  it("returns tier 0 for exact match ignoring case", () => {
    expect(scoreMatch("pi", "pi")).toEqual({ tier: 0, offset: 0 });
    expect(scoreMatch("PI", "pi")).toEqual({ tier: 0, offset: 0 });
    expect(scoreMatch("Pi", "pI")).toEqual({ tier: 0, offset: 0 });
  });

  it("returns tier 1 for whole-word match at start", () => {
    expect(scoreMatch("feat", "feat/pi-direct-sdk")).toEqual({
      tier: 1,
      offset: 0,
    });
  });

  it("returns tier 1 for whole-word match in middle", () => {
    expect(scoreMatch("pi", "feat/pi-direct-sdk")).toEqual({
      tier: 1,
      offset: 5,
    });
    expect(scoreMatch("pi", "a b pi c d")).toEqual({ tier: 1, offset: 4 });
  });

  it("returns tier 2 for string prefix that does not complete a word", () => {
    expect(scoreMatch("par", "party")).toEqual({ tier: 2, offset: 0 });
  });

  it("returns tier 3 for word-boundary start that does not complete a word", () => {
    expect(scoreMatch("par", "a/party")).toEqual({ tier: 3, offset: 2 });
  });

  it("returns tier 4 for substring inside a word", () => {
    expect(scoreMatch("art", "party")).toEqual({ tier: 4, offset: 1 });
  });

  it("does not assemble one query word across whitespace-separated words", () => {
    for (const separator of [" ", "\t", "\n", " "]) {
      expect(scoreMatch("terminal", `term${separator}inal`)).toBeNull();
    }
  });

  it("keeps subsequences within a word, including names and later words", () => {
    expect(scoreMatch("trmnl", "Fix terminal resizing")).toEqual({
      tier: 5,
      offset: 4,
      spread: 8,
    });
    expect(scoreMatch("abc", "AgentBrowserConfig")?.tier).toBe(5);
    expect(scoreMatch("pasbab", "paseo-babysit")?.tier).toBe(5);
    expect(scoreMatch("confg", "check configuration")?.offset).toBe(6);
  });

  it("returns null when query is not found", () => {
    expect(scoreMatch("xyz", "feat/pi-direct-sdk")).toBeNull();
  });

  it("picks the best tier across multiple occurrences", () => {
    expect(scoreMatch("ab", "xabxab-yy")).toEqual({ tier: 4, offset: 1 });
    expect(scoreMatch("ab", "xxab x-ab-y")).toEqual({ tier: 1, offset: 7 });
  });

  it("prefers earlier offset when tiers are equal", () => {
    expect(scoreMatch("pi", "pi-a pi-b")).toEqual({ tier: 1, offset: 0 });
  });

  it("treats common separators as word boundaries", () => {
    for (const text of ["x/pi", "x-pi", "x_pi", "x pi", "x.pi", "x:pi"]) {
      expect(scoreMatch("pi", text)).toEqual({ tier: 1, offset: 2 });
    }
    expect(scoreMatch("202", "#202 feat")).toEqual({ tier: 1, offset: 1 });
  });
});

describe("scorePathMatch", () => {
  it("matches a literal fragment anywhere in the complete path", () => {
    expect(
      scorePathMatch(
        "skills/",
        "something/something-else/skills/paseo-advisor/SKILL.md",
      ),
    ).not.toBeNull();
  });

  it("matches spaced text against a compact path", () => {
    expect(scorePathMatch("blank page editor", "blankpage/editor")).toEqual({
      tier: 0,
      offset: 0,
    });
  });

  it("keeps the original path offset for compact matches", () => {
    expect(
      scorePathMatch("blank page editor", "projects/blankpage/editor"),
    ).toEqual({ tier: 4, offset: 9 });
  });
});
