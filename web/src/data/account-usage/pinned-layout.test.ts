import { describe, expect, it } from "vitest";

import {
  quotaPinnedLayout,
  type QuotaLayoutMeasurements,
} from "./pinned-layout";

const FIT: QuotaLayoutMeasurements = {
  availableWidth: 100,
  labelsWidth: 40,
  percentsWidth: 20,
  windowCount: 3,
};

describe("one-line account quota summary layout", () => {
  it("shows labels until every measured width is known", () => {
    expect(quotaPinnedLayout({ ...FIT, availableWidth: null })).toBe("labels");
    expect(quotaPinnedLayout({ ...FIT, labelsWidth: null })).toBe("labels");
    expect(quotaPinnedLayout({ ...FIT, percentsWidth: null })).toBe("labels");
  });

  it("admits every meter exactly when labels plus three 16px tracks and 4px gaps fit", () => {
    expect(quotaPinnedLayout(FIT)).toBe("meters");
    expect(quotaPinnedLayout({ ...FIT, availableWidth: 99 })).toBe("labels");
    expect(
      quotaPinnedLayout({ ...FIT, availableWidth: 80, windowCount: 2 }),
    ).toBe("meters");
  });

  it("uses labels alone when meters do not fit, including the exact label edge", () => {
    expect(quotaPinnedLayout({ ...FIT, availableWidth: 40 })).toBe("labels");
  });

  it("uses percentages when they are the smaller representation", () => {
    expect(quotaPinnedLayout({ ...FIT, availableWidth: 39 })).toBe("percents");
    expect(
      quotaPinnedLayout({ ...FIT, availableWidth: 10, percentsWidth: 60 }),
    ).toBe("labels");
    expect(
      quotaPinnedLayout({ ...FIT, availableWidth: 10, percentsWidth: 40 }),
    ).toBe("labels");
  });

  it("treats invalid observer measurements or counts as not measured", () => {
    for (const width of [Number.NaN, Number.POSITIVE_INFINITY, -1]) {
      expect(quotaPinnedLayout({ ...FIT, availableWidth: width })).toBe(
        "labels",
      );
    }
    for (const windowCount of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
      expect(quotaPinnedLayout({ ...FIT, windowCount })).toBe("labels");
    }
  });
});
