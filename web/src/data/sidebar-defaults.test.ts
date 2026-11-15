// Ending an agent does not archive its conversation. The bridge and
// page defaults must expose retained work while respecting a saved off.
import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import { fixtureId } from "../testing/ids";
import { liveSession } from "./fixtures/machines";
import { firstPaint } from "./prefs";
import { applyChange, DEFAULT_PREFERENCES } from "./prefs-change";
import { DEFAULT_VIEW, listed } from "./sidebar";

const stopped: readonly SessionInfo[] = [
  { ...liveSession, status: { state: "exited", code: 0 } },
  {
    ...liveSession,
    id: fixtureId("SessionId", "interrupted"),
    status: { state: "interrupted" },
  },
  {
    ...liveSession,
    id: fixtureId("SessionId", "archived"),
    status: { state: "exited", code: 0 },
    marks: { archived_at: "2026-11-14T12:00:00Z" },
  },
];

describe("ended conversations before and after a choice", () => {
  it("uses one on default for the first paint and the sidebar fallback", () => {
    expect(DEFAULT_VIEW).toBe(DEFAULT_PREFERENCES.sidebar);
    expect(firstPaint({}).sidebar.ended).toBe(true);
    expect(listed(stopped, DEFAULT_VIEW).map((item) => item.id)).toEqual([
      liveSession.id,
      "interrupted",
    ]);
  });

  it("keeps an explicit off through an unrelated change, and can turn on again", () => {
    const off = applyChange(DEFAULT_PREFERENCES, {
      set: "sidebar_ended",
      value: false,
    });
    const changed = applyChange(off, { set: "theme", value: "dark" });
    expect(changed.sidebar.ended).toBe(false);
    expect(listed(stopped, changed.sidebar)).toEqual([]);
    const on = applyChange(changed, { set: "sidebar_ended", value: true });
    expect(listed(stopped, on.sidebar)).toHaveLength(2);
  });
});
