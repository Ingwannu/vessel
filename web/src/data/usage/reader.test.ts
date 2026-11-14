import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { UsageReport } from "../../wire/proto";
import { usageReport } from "../fixtures/usage";
import type { Outcome } from "../outcome";
import { UsageReader, usageFor, type UsageSelection } from "./reader";

function deferred() {
  let complete: (value: Outcome<UsageReport>) => void = () => {
    throw new Error("deferred result not initialized");
  };
  const promise = new Promise<Outcome<UsageReport>>((resolve) => {
    complete = resolve;
  });
  return {
    promise,
    complete: (value: Outcome<UsageReport>) => {
      complete(value);
    },
  };
}
const first: UsageSelection = {
  node: fixtureId("NodeKey", "first"),
  period: "day",
};
const second: UsageSelection = {
  node: fixtureId("NodeKey", "second"),
  period: "week",
};

/** An event, not a timer, ends the test when the reader publishes the expected state. */
function ready(reader: UsageReader): Promise<void> {
  return new Promise((resolve) => {
    const stop = reader.store.subscribe(() => {
      if (reader.store.get().kind !== "loading") {
        stop();
        resolve();
      }
    });
  });
}

describe("one visible usage query at a time", () => {
  it("hides the previous selection before a render effect starts its read", () => {
    const read = {
      kind: "ready",
      selection: first,
      report: usageReport(),
    } as const;
    expect(usageFor(read, { ...first })).toBe(read);
    expect(usageFor(read, second)).toEqual({
      kind: "loading",
      selection: second,
    });
    const week = { ...first, period: "week" } as const;
    expect(usageFor(read, week)).toEqual({ kind: "loading", selection: week });
  });

  it("replaces intermediate selections and never publishes a stale machine", async () => {
    const held = deferred();
    const calls: UsageSelection[] = [];
    const report = usageReport();
    const reader = new UsageReader((selection) => {
      calls.push(selection);
      return calls.length === 1
        ? held.promise
        : Promise.resolve({ ok: true, value: report });
    });
    reader.request(first);
    reader.request({ ...first, period: "week" });
    reader.request(second);
    expect(calls).toEqual([first]);
    expect(reader.store.get()).toEqual({ kind: "loading", selection: second });
    const done = ready(reader);
    held.complete({ ok: true, value: usageReport() });
    await done;
    expect(calls).toEqual([first, second]);
    expect(reader.store.get()).toEqual({
      kind: "ready",
      selection: second,
      report,
    });
  });

  it("stops queued work and discards a result from an unmounted page", async () => {
    const held = deferred();
    const calls: UsageSelection[] = [];
    const reader = new UsageReader((selection) => {
      calls.push(selection);
      return held.promise;
    });
    reader.request(first);
    reader.request(second);
    reader.stop();
    held.complete({ ok: true, value: usageReport() });
    await held.promise;
    expect(calls).toEqual([first]);
    expect(reader.store.get()).toEqual({ kind: "idle" });
  });

  it("does not revive a stopped lifetime when the caller reuses its selection object", async () => {
    const held = deferred();
    const report = usageReport();
    const published: UsageReport[] = [];
    let calls = 0;
    const reader = new UsageReader(() => {
      calls += 1;
      return calls === 1
        ? held.promise
        : Promise.resolve({ ok: true, value: report });
    });
    reader.store.subscribe(() => {
      const read = reader.store.get();
      if (read.kind === "ready") published.push(read.report);
    });
    reader.request(first);
    reader.stop();
    reader.request(first);
    const done = ready(reader);
    const stale = usageReport();
    stale.totals.reported = 99;
    held.complete({ ok: true, value: stale });
    await done;
    expect(calls).toBe(2);
    expect(published).toEqual([report]);
  });

  it("keeps a typed failure and refreshes after it without fabricating zeros", async () => {
    const problem = {
      from: "op",
      reason: "busy",
      message: "Try again",
    } as const;
    const report = usageReport();
    let calls = 0;
    const reader = new UsageReader(() => {
      calls += 1;
      return Promise.resolve(
        calls === 1 ? { ok: false, problem } : { ok: true, value: report },
      );
    });
    let done = ready(reader);
    reader.request(first);
    await done;
    expect(reader.store.get()).toEqual({
      kind: "failed",
      selection: first,
      problem,
    });
    done = ready(reader);
    reader.request(first);
    await done;
    expect(reader.store.get()).toEqual({
      kind: "ready",
      selection: first,
      report,
    });
  });
});
