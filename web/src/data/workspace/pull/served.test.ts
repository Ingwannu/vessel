import { describe, expect, it } from "vitest";

import { pullPaneServed, pullReach } from "./served";

describe("the pull request pane's machines", () => {
  it("is served from protocol 39 only", () => {
    expect(pullPaneServed(38)).toBe(false);
    expect(pullPaneServed(39)).toBe(true);
    expect(pullPaneServed(41)).toBe(true);
    expect(pullPaneServed(null)).toBe(false);
  });

  it("tells a machine not connected from one too old", () => {
    expect(pullReach(null)).toBe("offline");
    expect(pullReach(38)).toBe("old");
    expect(pullReach(39)).toBe("served");
  });
});
