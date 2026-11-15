import { describe, expect, it } from "vitest";

import { followAfterScroll, NEAR_END, type EndView } from "./follow-end";

const before: EndView = { top: 0, height: 660, width: 900, content: 660 };

describe("end intent when layout and scroll arrive together", () => {
  it("keeps the end when the scroll from a narrower, shorter view arrives before resize delivery", () => {
    const after = { top: 0, height: 300, width: 390, content: 400 };
    expect(followAfterScroll({ before, after, following: true })).toBe("end");
    expect(followAfterScroll({ before, after, following: false })).toBe("away");
  });

  it("keeps the end when wrapped rows grow after the viewport was measured", () => {
    const old = { top: 100, height: 300, width: 390, content: 400 };
    const after = { ...old, content: old.content + NEAR_END + 1 };
    expect(followAfterScroll({ before: old, after, following: true })).toBe(
      "end",
    );
  });

  it("honors an upward move, including when it coincides with layout", () => {
    const old = { top: 500, height: 300, width: 900, content: 800 };
    const after = { ...old, top: 300 };
    expect(followAfterScroll({ before: old, after, following: true })).toBe(
      "away",
    );
    expect(
      followAfterScroll({
        before: old,
        after: { ...after, width: 390 },
        following: true,
      }),
    ).toBe("away");
  });

  it("keeps a browser clamp to the new end, and preserves the near-end boundary", () => {
    const old = { top: 500, height: 300, width: 900, content: 800 };
    expect(
      followAfterScroll({
        before: old,
        after: { ...old, top: 100, content: 400 },
        following: true,
      }),
    ).toBe("end");
    expect(
      followAfterScroll({
        before: old,
        after: { ...old, top: old.top - NEAR_END },
        following: true,
      }),
    ).toBe("away");
    expect(
      followAfterScroll({
        before: old,
        after: { ...old, top: old.top - NEAR_END + 1 },
        following: true,
      }),
    ).toBe("near");
  });
});
