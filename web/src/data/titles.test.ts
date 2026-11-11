import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import {
  checkTitle,
  hasTitle,
  sessionName,
  TITLE_MAX,
  titleFromPrompt,
} from "./titles";

const SESSION: SessionInfo = {
  id: "8e7f7d69-813c-4f55-9a36-48f03226cfd1",
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
    expect(checkTitle("x".repeat(TITLE_MAX)).ok).toBe(true);
    expect(checkTitle("x".repeat(TITLE_MAX + 1)).ok).toBe(false);
  });

  it("counts characters, not UTF-16 units", () => {
    expect(checkTitle("한".repeat(TITLE_MAX)).ok).toBe(true);
    expect(checkTitle("🙂".repeat(TITLE_MAX)).ok).toBe(true);
  });
});

describe("titleFromPrompt", () => {
  it("takes the first line with text, cut at 80 characters", () => {
    expect(titleFromPrompt("\n\n  Tidy the overview \nand more")).toBe(
      "Tidy the overview",
    );
    expect(titleFromPrompt("a".repeat(100))).toBe("a".repeat(80));
    expect(titleFromPrompt("  \n ")).toBeNull();
  });
});
