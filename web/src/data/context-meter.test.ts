import { expect, test } from "vitest";

import { costLabel, meterOf, tokens, turnTokens } from "./context-meter";

test("no meter before the agent says, or without a window", () => {
  expect(meterOf(null)).toBeNull();
  expect(meterOf({ used: 10, size: 0 })).toBeNull();
});

test("the ring, its colour and its words, by the reference's thresholds", () => {
  const room = meterOf({ used: 1200, size: 200_000 });
  expect(room).toMatchObject({ percent: 1, fullness: "room", arc: 0.08 });
  expect(room).toMatchObject({
    title: "Context window",
    used: "1% used",
    details: ["1k / 200k tokens"],
  });
  expect(meterOf({ used: 140_000, size: 200_000 })?.fullness).toBe("low");
  const full = meterOf({
    used: 190_000,
    size: 200_000,
    cost: { amount: 0.5, unit: "USD" },
  });
  expect(full?.fullness).toBe("full");
  expect(full?.details).toContain("Session cost $0.50");
});

test("counts and costs as written", () => {
  expect(tokens(950)).toBe("950");
  expect(tokens(84_400)).toBe("84k");
  expect(tokens(1_200_000)).toBe("1.2M");
  expect(costLabel(12.5, "credits")).toBe("12.5 credits");
  expect(turnTokens({ total: 30, input: 20, output: 10 })).toBe(
    "30 tokens · 20 in · 10 out",
  );
});
