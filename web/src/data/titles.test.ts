import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import {
  checkTitle,
  hasTitle,
  LEGACY_TITLE_MAX,
  LONG_TITLES_PROTOCOL,
  sessionName,
  TITLE_MAX,
  titleFromPrompt,
  titleMax,
} from "./titles";
import { fixtureId } from "../testing/ids";

const SESSION: SessionInfo = {
  id: fixtureId("SessionId", "8e7f7d69-813c-4f55-9a36-48f03226cfd1"),
  spec: { harness: "claude-code", mode: "structured", cwd: "/work/vessel" },
  created_by: { kind: "owner" },
  status: { state: "running" },
  created_at: "2026-11-11T08:00:00Z",
  next_seq: 4,
};

describe("sessionName", () => {
  it("takes the log's rename, then the node's title, then the folder", () => {
    expect(sessionName(SESSION)).toBe("vessel");
    expect(hasTitle(SESSION)).toBe(false);
    const titled = { ...SESSION, title: "Fix the fleet overview" };
    expect(sessionName(titled)).toBe("Fix the fleet overview");
    expect(hasTitle(titled)).toBe(true);
    expect(sessionName(titled, "Renamed")).toBe("Renamed");
  });
});

describe("checkTitle", () => {
  it("trims, and refuses what the node refuses", () => {
    expect(checkTitle("  Release notes  ")).toEqual({
      ok: true,
      title: "Release notes",
    });
    expect(checkTitle("   ").ok).toBe(false);
    expect(checkTitle("two\nlines").ok).toBe(false);
    expect(checkTitle("x".repeat(TITLE_MAX), TITLE_MAX).ok).toBe(true);
    expect(checkTitle("x".repeat(TITLE_MAX + 1), TITLE_MAX).ok).toBe(false);
  });

  it("allows 200 on a node of LONG_TITLES_PROTOCOL and 120 before it", () => {
    const long = "x".repeat(150);
    expect(titleMax(LONG_TITLES_PROTOCOL)).toBe(TITLE_MAX);
    expect(titleMax(LONG_TITLES_PROTOCOL - 1)).toBe(LEGACY_TITLE_MAX);
    expect(titleMax(null)).toBe(LEGACY_TITLE_MAX);
    expect(checkTitle(long, titleMax(LONG_TITLES_PROTOCOL)).ok).toBe(true);
    expect(checkTitle(long, titleMax(LONG_TITLES_PROTOCOL - 1))).toEqual({
      ok: false,
      reason: "A title is at most 120 characters.",
    });
    expect(checkTitle(long).ok).toBe(false);
  });

  it("counts characters, not UTF-16 units", () => {
    expect(checkTitle("한".repeat(TITLE_MAX), TITLE_MAX).ok).toBe(true);
    expect(checkTitle("🙂".repeat(TITLE_MAX), TITLE_MAX).ok).toBe(true);
  });
});

describe("titleFromPrompt", () => {
  it("takes the first line with text, its blanks folded, cut at 60", () => {
    expect(titleFromPrompt("\n\n  Tidy the overview \nand more")).toBe(
      "Tidy the overview",
    );
    expect(titleFromPrompt("Fix   the\t\tbuild\u2003now")).toBe(
      "Fix the build now",
    );
    expect(titleFromPrompt("a".repeat(100))).toBe("a".repeat(60));
    expect(titleFromPrompt("word ".repeat(20))).toBe(
      "word ".repeat(12).trimEnd(),
    );
    expect(titleFromPrompt("  \n ")).toBeNull();
  });
});
