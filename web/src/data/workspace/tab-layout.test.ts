import { describe, expect, it } from "vitest";

import { sessionIdOf } from "../../bridge/ids";
import {
  addDiscovered,
  EMPTY_LAYOUT,
  enterRoute,
  moveTab,
  removeTab,
  showTab,
  tabKey,
  tabsToClose,
} from "./tab-layout";

const first = sessionIdOf("00000000-0000-4000-8000-000000000001");
const second = sessionIdOf("00000000-0000-4000-8000-000000000002");

if (first === null || second === null) {
  throw new Error("Invalid fixture session ID");
}

describe("workspace tab ordering and routes", () => {
  it("restores passive selection for the same route and reveals a new route", () => {
    const initial = enterRoute(EMPTY_LAYOUT, first);
    const files = showTab(initial, { kind: "files" });
    expect(enterRoute(files, first)).toBe(files);
    const another = enterRoute(files, second);
    expect(another.active).toBe(`agent:${second}`);
    expect(showTab(another, { kind: "files" }).tabs).toBe(another.tabs);
  });

  it("reveals an agent whose own route is re-entered after closing its tab", () => {
    const initial = showTab(enterRoute(EMPTY_LAYOUT, first), { kind: "files" });
    const key = `agent:${first}`;
    const closed = removeTab(initial, key);
    expect(closed.active).toBe("files");
    expect(addDiscovered(closed, [{ kind: "agent", id: first }])).toBe(closed);
    const reopened = enterRoute(closed, first);
    expect(reopened.active).toBe(key);
    expect(reopened.hidden).toEqual([]);
  });

  it("keeps selection while moving, scopes bulk targets, and keeps a closed agent hidden", () => {
    const initial = showTab(enterRoute(EMPTY_LAYOUT, first), { kind: "files" });
    const ready = addDiscovered(initial, [
      { kind: "agent", id: second },
      { kind: "diff" },
    ]);
    const order = moveTab(ready, "diff", "files", "before");
    expect(order.tabs.map(tabKey)).toEqual([
      `agent:${first}`,
      "diff",
      "files",
      `agent:${second}`,
    ]);
    expect(order.active).toBe("files");
    expect(tabsToClose(order.tabs, "files", "before").map(tabKey)).toEqual([
      `agent:${first}`,
      "diff",
    ]);
    expect(tabsToClose(order.tabs, "files", "after").map(tabKey)).toEqual([
      `agent:${second}`,
    ]);
    const closed = removeTab(order, `agent:${second}`);
    expect(addDiscovered(closed, [{ kind: "agent", id: second }])).toBe(closed);
    expect(removeTab(closed, "files").active).toBe("diff");
  });
});
