import { describe, expect, it } from "vitest";

import type { BookNode } from "../wire/bridge";
import type { SessionInfo, SessionStatus } from "../wire/proto";
import { paletteEntries } from "./commands";
import type { FleetState } from "./fleet";
import { fixtureId } from "../testing/ids";

function session(
  id: string,
  at: string,
  status: SessionStatus,
  cwd: string,
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd },
    created_by: { kind: "owner" },
    status,
    created_at: at,
    next_seq: 0,
  };
}

const laptop: BookNode = {
  key: fixtureId("NodeKey", "laptop"),
  label: "laptop",
  address: "ws://127.0.0.1:7001/rpc",
  transport: "ws",
  default: true,
  id: null,
};

function state(): FleetState {
  return {
    link: { kind: "open", version: "0.1.0" },
    book: [laptop],
    bookError: null,
    statuses: {},
    harnesses: {},
    profiles: {},
    models: {},
    agents: {},
    errors: {},
    selected: null,
    sessions: {
      [fixtureId("NodeKey", "laptop")]: [
        session(
          "aaaa1111",
          "2026-11-07T09:00:00Z",
          { state: "exited", code: 0 },
          "/w/old",
        ),
        session(
          "bbbb2222",
          "2026-11-07T10:00:00Z",
          { state: "running" },
          "/w/api",
        ),
        session(
          "cccc3333",
          "2026-11-07T11:00:00Z",
          { state: "running" },
          "/w/web",
        ),
      ],
    },
  };
}

describe("paletteEntries", () => {
  it("lists the actions, the places, the machines, the sessions, the theme", () => {
    const groups = paletteEntries(state(), "laptop").map(
      (entry) => entry.group,
    );
    expect([...new Set(groups)]).toEqual([
      "Actions",
      "Go to",
      "Machines",
      "Sessions",
      "Theme",
    ]);
  });

  it("starts a new session on the selected machine", () => {
    const action = paletteEntries(state(), "laptop").find(
      (entry) => entry.id === "new-session",
    );
    expect(action?.target).toEqual({ kind: "new-session", node: "laptop" });
  });

  it("offers to add a machine, wherever the palette is opened", () => {
    const action = paletteEntries(state(), null).find(
      (entry) => entry.id === "add-machine",
    );
    expect(action).toMatchObject({
      label: "Add a machine",
      group: "Actions",
      target: { kind: "add-machine" },
    });
  });

  it("puts live sessions first, newest first, and ended ones after", () => {
    const sessions = paletteEntries(state(), null)
      .filter((entry) => entry.group === "Sessions")
      .map((entry) => entry.label);
    expect(sessions).toEqual(["web", "api", "old"]);
  });

  it("opens a session at its route and finds it by its full id", () => {
    const entry = paletteEntries(state(), null).find(
      (candidate) => candidate.keywords === "bbbb2222",
    );
    expect(entry?.target).toEqual({
      kind: "route",
      to: "/sessions/laptop/bbbb2222",
    });
    expect(entry?.detail).toBe("running · /w/api");
  });

  it("opens a machine's detail in Settings, in the page's words", () => {
    const entry = paletteEntries(state(), null).find(
      (candidate) => candidate.group === "Machines",
    );
    expect(entry).toMatchObject({
      label: "laptop",
      target: { kind: "route", to: "/settings/machines?machine=laptop" },
    });
    expect(entry?.detail).not.toMatch(/ws:|node/);
  });

  it("goes to the board and to each section of Settings", () => {
    const places = paletteEntries(state(), null)
      .filter((entry) => entry.group === "Go to")
      .map((entry) => entry.label);
    expect(places).toEqual([
      "Board",
      "Schedules",
      "Settings: General",
      "Settings: Appearance",
      "Settings: Terminal",
      "Settings: Editor",
      "Settings: Files",
      "Settings: Shortcuts",
      "Settings: Notifications",
      "Settings: Agents",
      "Settings: Models",
      "Settings: Machines",
      "Settings: Projects",
      "Settings: Devices",
      "Settings: Usage",
      "Settings: Terminals",
      "Settings: Sync",
      "Settings: Plugins",
      "Settings: Diagnostics",
      "Settings: About",
    ]);
  });

  it("routes to schedules using the same localized label as the sidebar", () => {
    const entry = paletteEntries(state(), null, "ko-KR").find(
      (candidate) => candidate.id === "schedules",
    );
    expect(entry).toMatchObject({
      label: "일정",
      target: { kind: "route", to: "/schedules" },
    });
    expect(entry?.keywords).toContain("recurring");
  });

  it("switches the theme three ways", () => {
    const themes = paletteEntries(state(), null).filter(
      (entry) => entry.group === "Theme",
    );
    expect(themes.map((entry) => [entry.label, entry.target])).toEqual([
      ["Theme: system", { kind: "theme", preference: "system" }],
      ["Theme: light", { kind: "theme", preference: "light" }],
      ["Theme: dark", { kind: "theme", preference: "dark" }],
    ]);
  });
});
