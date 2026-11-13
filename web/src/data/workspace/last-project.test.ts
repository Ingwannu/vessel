import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { ProjectInfo, SessionInfo } from "../../wire/proto";
import { lastProject } from "./last-project";
import type { ProjectsView } from "./project-store";

function project(id: string): ProjectInfo {
  return {
    id: fixtureId("ProjectId", id),
    name: id,
    root: `/work/${id}`,
    git: true,
    default_branch: "origin/main",
    state: { state: "ready" },
    created_at: "2026-11-12T00:00:00Z",
    revision: 1,
  };
}

const PROJECTS: ProjectsView = {
  kind: "ready",
  list: {
    git: { state: "found", version: { major: 2, minor: 47, patch: 0 } },
    projects: [project("shop"), project("notes")],
    more: false,
  },
};

function session(
  id: string,
  at: string,
  inProject: string | null,
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: `/work/${id}` },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: at,
    next_seq: 0,
    ...(inProject === null
      ? {}
      : { workspace: { project: fixtureId("ProjectId", inProject) } }),
  };
}

describe("the project a new session starts in", () => {
  it("is the project of the machine's most recently active session", () => {
    const sessions = [
      session("a", "2026-11-13T01:00:00Z", "shop"),
      session("b", "2026-11-13T03:00:00Z", "notes"),
      session("c", "2026-11-13T04:00:00Z", null),
    ];
    expect(lastProject(sessions, PROJECTS)).toBe("notes");
  });

  it("passes over a project the machine no longer has", () => {
    const sessions = [
      session("a", "2026-11-13T01:00:00Z", "shop"),
      session("b", "2026-11-13T03:00:00Z", "removed"),
    ];
    expect(lastProject(sessions, PROJECTS)).toBe("shop");
  });

  it("is none while the projects are not read, or with no session in one", () => {
    const sessions = [session("a", "2026-11-13T01:00:00Z", "shop")];
    expect(lastProject(sessions, { kind: "loading" })).toBeNull();
    expect(lastProject(undefined, PROJECTS)).toBeNull();
    expect(
      lastProject([session("c", "2026-11-13T04:00:00Z", null)], PROJECTS),
    ).toBeNull();
  });
});
