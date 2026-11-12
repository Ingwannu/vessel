import { describe, expect, it } from "vitest";

import type { FleetMember, Manifest } from "../wire/proto";
import { memberLine, memberState, secretNames, withConflicts } from "./sync";
import { fixtureId } from "../testing/ids";

const member: FleetMember = {
  device: fixtureId("DeviceId", "d-1"),
  label: "gpu-box",
  applied: 3,
  conflicts: [],
  reported_at: "2026-11-11T06:59:00Z",
};

describe("a member of what a machine shares", () => {
  it("is up to date, behind by some versions, or not reported yet", () => {
    expect(memberState(member, 3)).toEqual({ word: "Up to date", tone: "ok" });
    expect(memberState(member, 5)).toEqual({ word: "2 behind", tone: "warn" });
    expect(memberState({ ...member, applied: null }, 5).word).toBe("Not yet");
  });

  it("says what it applied, its kept edits and its last report", () => {
    expect(memberLine(member, () => "1m")).toEqual([
      "Applied version 3",
      "no kept edits",
      "reported 1m ago",
    ]);
    const edited = { ...member, conflicts: ["AGENTS.md"] };
    expect(memberLine(edited, () => "1m")[1]).toBe("1 kept edit");
    expect(withConflicts([member, edited])).toEqual([edited]);
  });
});

describe("the secrets the shared tool servers name", () => {
  it("are names only, each once, sorted", () => {
    const manifest: Manifest = {
      generation: 1,
      files: {},
      mcp: {
        search: {
          type: "stdio",
          command: "search-mcp",
          env: { KEY: { secret: "SEARCH_KEY" }, MODE: "fast" },
        },
        docs: {
          type: "http",
          url: "https://docs.example/mcp",
          headers: { Authorization: { secret: "DOCS_TOKEN" } },
        },
      },
    };
    expect(secretNames(manifest)).toEqual(["DOCS_TOKEN", "SEARCH_KEY"]);
  });
});
