import { describe, expect, test } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  RegistrySnapshot,
  SessionInfo,
  WorkspaceDescriptor,
} from "../../wire/proto";
import { bookNode, connectedAs } from "../fixtures/machines";
import type { FleetState } from "../fleet";
import { DEFAULT_VIEW } from "../sidebar";
import { sidebarWorkspaces, workspaceOf } from "./groups";
import type { LabelsState } from "./hub";
import { UNLABELLED } from "./names";

const laptop = fixtureId("NodeKey", "laptop");
const app = fixtureId("ProjectId", "app");
const docs = fixtureId("ProjectId", "docs");

function workspace(
  id: string,
  project: typeof app,
  extra: Partial<WorkspaceDescriptor> = {},
): WorkspaceDescriptor {
  return {
    id: fixtureId("WorkspaceId", id),
    project,
    path: `/work/${id}`,
    title: null,
    automation: { kind: "allowed" },
    pin: "unpinned",
    backing: { kind: "local" },
    lifecycle: { kind: "active" },
    visibility: "foreground",
    created_at: "2026-11-16T00:00:00Z",
    revision: 0,
    ...extra,
  };
}

function session(
  id: string,
  at: string,
  project: typeof app | null,
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: `/work/${id}` },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: at,
    next_seq: 0,
    ...(project === null ? {} : { workspace: { project } }),
  };
}

const SNAPSHOT: RegistrySnapshot = {
  node: fixtureId("NodeId", "n-1"),
  revision: 3,
  workspaces: [
    workspace("app-root", app),
    workspace("app-newer", app, { created_at: "2026-11-16T01:00:00Z" }),
    workspace("docs-root", docs, { pin: "pinned" }),
    workspace("gone", docs, {
      lifecycle: { kind: "archived", at: "2026-11-16T02:00:00Z" },
    }),
  ],
  memberships: [
    {
      workspace: fixtureId("WorkspaceId", "app-newer"),
      resource: { kind: "session", session: fixtureId("SessionId", "s-old") },
    },
  ],
  labels: [
    { workspace: fixtureId("WorkspaceId", "app-root"), labels: ["Blocked"] },
  ],
};

const FLEET: FleetState = {
  link: { kind: "open", version: "0.1.0" },
  book: [bookNode("laptop")],
  bookError: null,
  statuses: { [laptop]: connectedAs("laptop") },
  harnesses: {},
  profiles: {},
  models: {},
  sessions: {
    [laptop]: [
      session("s-new", "2026-11-16T05:00:00Z", app),
      session("s-old", "2026-11-16T04:00:00Z", app),
      session("s-loose", "2026-11-16T03:00:00Z", null),
    ],
  },
  agents: {},
  errors: {},
  selected: null,
};

const LABELS: LabelsState = {
  registries: { [laptop]: SNAPSHOT },
  catalogs: {
    [laptop]: {
      state: "online",
      labels: [{ name: "Blocked", color: "red" }],
      error: null,
    },
  },
};

describe("which workspace a session belongs to", () => {
  test("the snapshot's membership first", () => {
    const old = FLEET.sessions[laptop]?.[1];
    expect(old && workspaceOf(SNAPSHOT, old)).toBe("app-newer");
  });

  test("a newer session at a project root joins its oldest workspace", () => {
    const fresh = FLEET.sessions[laptop]?.[0];
    expect(fresh && workspaceOf(SNAPSHOT, fresh)).toBe("app-root");
  });
});

describe("the sidebar grouped by workspace", () => {
  test("pinned first, archived left out, loose sessions last", () => {
    const rows = sidebarWorkspaces(FLEET, LABELS, DEFAULT_VIEW, []);
    expect(rows.map((row) => row.title)).toEqual([
      "docs-root",
      "app-root",
      "app-newer",
      "Other sessions",
    ]);
    expect(rows[1]?.chips).toEqual([{ name: "Blocked", color: "red" }]);
    expect(rows[1]?.sessions.map((each) => each.id)).toEqual(["s-new"]);
  });

  test("a label pick keeps carriers and drops the loose group", () => {
    const rows = sidebarWorkspaces(FLEET, LABELS, DEFAULT_VIEW, ["blocked"]);
    expect(rows.map((row) => row.title)).toEqual(["app-root"]);
  });

  test("Unlabelled keeps the workspaces carrying none", () => {
    const rows = sidebarWorkspaces(FLEET, LABELS, DEFAULT_VIEW, [UNLABELLED]);
    expect(rows.map((row) => row.title)).toEqual(["docs-root", "app-newer"]);
  });

  test("a machine whose registry has not answered lists its sessions loose", () => {
    const rows = sidebarWorkspaces(
      FLEET,
      { registries: {}, catalogs: {} },
      DEFAULT_VIEW,
      [],
    );
    expect(rows.map((row) => row.title)).toEqual(["Other sessions"]);
    expect(rows[0]?.sessions).toHaveLength(3);
  });
});
