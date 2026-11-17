import { describe, expect, it } from "vitest";

import { QuotaPercent } from "./quota-percent";

describe("validated quota paint value", () => {
  it("clamps finite percentages before paint and viewer-locale formatting", () => {
    expect(QuotaPercent.from(Number.POSITIVE_INFINITY)).toBeNull();
    expect(QuotaPercent.from(-20)?.format("en-US")).toBe("0%");
    expect(QuotaPercent.from(120)?.format("en-US")).toBe("100%");
    expect(QuotaPercent.from(20.5)?.format("en-US")).toBe("21%");
    expect(QuotaPercent.from(0)?.fraction).toBe(0);
    expect(QuotaPercent.from(100)?.fraction).toBe(1);
  });
});
