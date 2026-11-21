import { describe, expect, it } from "vitest";

import { pullPaneServed } from "./served";

describe("the pull request pane's machines", () => {
  it("is served from protocol 39 only", () => {
    expect(pullPaneServed(38)).toBe(false);
    expect(pullPaneServed(39)).toBe(true);
    expect(pullPaneServed(41)).toBe(true);
    expect(pullPaneServed(null)).toBe(false);
  });
});
