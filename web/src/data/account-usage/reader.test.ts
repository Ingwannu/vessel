import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  LimitRefresh,
  UsageReportLookup,
  UsageReports,
} from "../../wire/proto";
import type { Outcome } from "../outcome";
import { type AccountCalls, AccountReader } from "./reader";

const FIRST = fixtureId("NodeKey", "first");
const SECOND = fixtureId("NodeKey", "second");

function reports(policy: UsageReports["policy"]): UsageReports {
  return {
    at: "2026-11-16T09:00:00Z",
    policy,
    cost_visibility: "permitted",
    sources: [],
    reports: [],
    source_failures: [],
  };
}

const FAILED: Outcome<never> = {
  ok: false,
  problem: { from: "op", reason: "failed", message: "gone" },
};

/** Calls that answer only when the test says so, in order. */
function pending() {
  const reads: {
    node: string;
    refresh: LimitRefresh;
    answer: (value: Outcome<UsageReports>) => void;
  }[] = [];
  const lookups: ((value: Outcome<"on" | "off">) => void)[] = [];
  const calls: AccountCalls = {
    read: (node, refresh) =>
      new Promise((answer) => {
        reads.push({ node, refresh, answer });
      }),
    inspect: () =>
      Promise.resolve<Outcome<UsageReportLookup>>({
        ok: true,
        value: { kind: "off" },
      }),
    lookup: () =>
      new Promise((answer) => {
        lookups.push(answer);
      }),
    source: () => Promise.resolve(FAILED),
  };
  return { calls, reads, lookups };
}

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe("AccountReader", () => {
  it("reads cached on open and drops another machine's late answer", async () => {
    const { calls, reads } = pending();
    const reader = new AccountReader(calls);
    reader.open(FIRST);
    reader.open(SECOND);
    expect(reads.map((read) => [read.node, read.refresh])).toEqual([
      ["first", "cached"],
      ["second", "cached"],
    ]);
    reads[0]?.answer({ ok: true, value: reports("on") });
    await settle();
    expect(reader.store.get()).toEqual({ kind: "loading", node: SECOND });
    reads[1]?.answer({ ok: true, value: reports("off") });
    await settle();
    expect(reader.store.get()).toMatchObject({ kind: "ready", node: SECOND });
  });

  it("keeps the shown list when Refresh all fails", async () => {
    const { calls, reads } = pending();
    const reader = new AccountReader(calls);
    reader.open(FIRST);
    reads[0]?.answer({ ok: true, value: reports("on") });
    await settle();
    reader.refreshAll();
    expect(reader.store.get()).toMatchObject({ refreshing: true });
    // A second click while one runs sends nothing.
    reader.refreshAll();
    expect(reads).toHaveLength(2);
    expect(reads[1]?.refresh).toBe("refresh");
    reads[1]?.answer(FAILED);
    await settle();
    expect(reader.store.get()).toMatchObject({
      kind: "ready",
      refreshing: false,
      reports: { policy: "on" },
      problem: { message: "gone" },
    });
  });

  it("reads cached again after the switch, and not after stop", async () => {
    const { calls, reads, lookups } = pending();
    const reader = new AccountReader(calls);
    reader.open(FIRST);
    reads[0]?.answer({ ok: true, value: reports("off") });
    await settle();
    reader.setLookup("on");
    lookups[0]?.({ ok: true, value: "on" });
    await settle();
    expect(reads.map((read) => read.refresh)).toEqual(["cached", "cached"]);
    reader.setLookup("off");
    reader.stop();
    lookups[1]?.({ ok: true, value: "off" });
    await settle();
    expect(reads).toHaveLength(2);
    expect(reader.store.get()).toEqual({ kind: "idle" });
  });
});
