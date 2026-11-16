import { describe, expect, it } from "vitest";
import { quotaAmount, quotaPercent, quotaTone } from "./values";

describe("quota presentation values", () => {
  it("keeps independent sides and derives only an absent side", () => {
    expect(quotaPercent({ used: 20, remaining: 91 }, "used")?.percent).toBe(20);
    expect(
      quotaPercent({ used: 20, remaining: 91 }, "remaining")?.percent,
    ).toBe(91);
    expect(
      quotaPercent({ used: 20, remaining: null }, "remaining")?.percent,
    ).toBe(80);
    expect(quotaPercent({ used: null, remaining: 40 }, "used")?.percent).toBe(
      60,
    );
  });

  it("distinguishes unknown, zero and invalid reports", () => {
    expect(quotaPercent({ used: null, remaining: null }, "used")).toBeNull();
    expect(quotaPercent({ used: 0, remaining: null }, "used")?.percent).toBe(0);
    expect(
      quotaPercent({ used: 0, remaining: null }, "remaining")?.percent,
    ).toBe(100);
    expect(
      quotaPercent({ used: Number.NaN, remaining: 20 }, "used"),
    ).toBeNull();
  });

  it("preserves a supplied tone and the reference's exact thresholds", () => {
    expect(quotaTone({ used: 0, remaining: null }, null)).toBe("default");
    expect(quotaTone({ used: 69.9, remaining: null }, null)).toBe("default");
    expect(quotaTone({ used: 70, remaining: null }, null)).toBe("warning");
    expect(quotaTone({ used: 90, remaining: null }, null)).toBe("warning");
    expect(quotaTone({ used: 90.1, remaining: null }, null)).toBe("danger");
    expect(quotaTone({ used: 100, remaining: null }, "default")).toBe(
      "default",
    );
    expect(quotaTone({ used: null, remaining: null }, "ok")).toBe("ok");
  });

  it("keeps amount units, missing, zero and browser precision distinct", () => {
    expect(quotaAmount(null, "usd", "en-US")).toEqual({ kind: "unreported" });
    expect(quotaAmount(0, "usd", "en-US")).toEqual({
      kind: "reported",
      text: "$0.00",
    });
    expect(quotaAmount(1250.5, "credits", "en-US")).toEqual({
      kind: "reported",
      text: "1,250.5",
    });
    expect(quotaAmount(-3, "requests", "en-US")).toEqual({
      kind: "reported",
      text: "-3",
    });
    expect(quotaAmount(1_200_000, "tokens", "en-US")).toEqual({
      kind: "reported",
      text: "1.2M",
    });
    expect(quotaAmount(1_200_000, "tokens", "ko-KR")).toEqual({
      kind: "reported",
      text: "120만",
    });
    expect(quotaAmount(Number.POSITIVE_INFINITY, "usd", "en-US")).toEqual({
      kind: "unrepresentable",
    });
    expect(quotaAmount(Number.MAX_SAFE_INTEGER + 1, "tokens", "en-US")).toEqual(
      { kind: "unrepresentable" },
    );
  });
});
