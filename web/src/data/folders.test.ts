import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import { folderSuggestions, MAX_FOLDERS } from "./folders";
import { fixtureId } from "../testing/ids";

function session(cwd: string | null, at: string): SessionInfo {
  return {
    id: fixtureId("SessionId", `${cwd ?? "home"}-${at}`),
    spec: {
      harness: "codex",
      mode: "terminal",
      args: [],
      size: { cols: 80, rows: 24 },
      cwd,
    },
    created_by: { kind: "owner" },
    status: { state: "exited", code: 0 },
    created_at: at,
    next_seq: 0,
  };
}

const NOW = Date.parse("2026-11-11T12:00:00Z");

describe("folderSuggestions", () => {
  it("lists each folder once, the latest used first", () => {
    const list = folderSuggestions(
      [
        session("/work/vessel", "2026-11-11T09:00:00Z"),
        session("/work/docs", "2026-11-11T11:00:00Z"),
        session("/work/vessel", "2026-11-11T11:30:00Z"),
        session(null, "2026-11-11T11:59:00Z"),
      ],
      NOW,
    );
    expect(list).toEqual([
      { value: "/work/vessel", detail: "30m ago" },
      { value: "/work/docs", detail: "1h ago" },
    ]);
  });

  it("keeps at most a dozen", () => {
    const many = Array.from({ length: 20 }, (_, index) =>
      session(
        `/work/p${String(index)}`,
        `2026-11-10T${String(index).padStart(2, "0")}:00:00Z`,
      ),
    );
    expect(folderSuggestions(many, NOW)).toHaveLength(MAX_FOLDERS);
  });
});
