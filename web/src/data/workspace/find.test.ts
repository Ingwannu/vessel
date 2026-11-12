import { describe, expect, it } from "vitest";

import { SEARCH_KEY_MAX } from "../../bridge/ids";
import { newSearchKey, nextSearchKey } from "./find";

describe("a field's search key", () => {
  it("counts from one and starts again after the largest the node takes", () => {
    expect(nextSearchKey(0)).toBe(1);
    expect(nextSearchKey(41)).toBe(42);
    expect(nextSearchKey(SEARCH_KEY_MAX)).toBe(1);
  });

  it("is different for each field that asks for one", () => {
    const composer = newSearchKey();
    const panel = newSearchKey();
    expect(panel).not.toBe(composer);
    expect(composer).toBeGreaterThan(0);
    expect(panel).toBeLessThanOrEqual(SEARCH_KEY_MAX);
  });
});
