import { describe, expect, it } from "vitest";

import type { LinkState } from "../bridge/socket";
import type { Outcome } from "./outcome";
import { liveBoundary } from "./terminal-live";

/** Reads the test answers by hand, in any order, and a link it drives. */
function harness() {
  const reads: ((outcome: Outcome<{ next_seq: number }>) => void)[] = [];
  const listeners = new Set<(link: LinkState) => void>();
  const boundary = liveBoundary({
    read: () =>
      new Promise((resolve) => {
        reads.push(resolve);
      }),
    onLink: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  });
  const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
  return {
    boundary,
    reads,
    listeners,
    /** Answer read `index` with `next_seq`. */
    answer: async (index: number, next: number) => {
      reads[index]?.({ ok: true, value: { next_seq: next } });
      await settle();
    },
    link: (link: LinkState) => {
      for (const listener of listeners) {
        listener(link);
      }
    },
  };
}

const OPEN: LinkState = { kind: "open", version: "test" };
const CLOSED: LinkState = { kind: "closed", retryInMs: 1000, attempt: 1 };

describe("the live boundary", () => {
  it("is history until the read answers, then live from its next_seq", async () => {
    const { boundary, reads, answer } = harness();
    expect(reads).toHaveLength(1);
    expect(boundary.written(40)).toBe("history");
    await answer(0, 40);
    expect(boundary.written(39)).toBe("history");
    expect(boundary.written(40)).toBe("live");
    expect(boundary.written(41)).toBe("live");
  });

  it("never answers what was written while the tab's link was down", async () => {
    const { boundary, reads, answer, link } = harness();
    await answer(0, 40);
    link(CLOSED);
    // The rejoin replays seq 40-49, written during the gap.
    expect(boundary.written(45)).toBe("history");
    link(OPEN);
    expect(reads).toHaveLength(2);
    expect(boundary.written(45)).toBe("history");
    await answer(1, 50);
    expect(boundary.written(49)).toBe("history");
    expect(boundary.written(50)).toBe("live");
  });

  it("never answers what was written while the bridge's stream reconnected", async () => {
    const { boundary, reads, answer } = harness();
    await answer(0, 10);
    boundary.stream({ state: "reconnecting", message: "node gone" });
    expect(boundary.written(12)).toBe("history");
    boundary.stream({ state: "resumed", at: 10 });
    expect(reads).toHaveLength(2);
    await answer(1, 20);
    expect(boundary.written(19)).toBe("history");
    expect(boundary.written(20)).toBe("live");
  });

  it("ignores a read that answers after a later gap began", async () => {
    const { boundary, answer, link } = harness();
    link(CLOSED);
    // The first read (sent before the gap) answers late: ignored.
    await answer(0, 5);
    expect(boundary.written(5)).toBe("history");
    link(OPEN);
    await answer(1, 30);
    expect(boundary.written(29)).toBe("history");
    expect(boundary.written(30)).toBe("live");
  });

  it("stays history when a read fails, until the next gap's read", async () => {
    const { boundary, reads, link, answer } = harness();
    reads[0]?.({
      ok: false,
      problem: {
        from: "call",
        failure: { kind: "disconnected" },
        message: "gone",
      },
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(boundary.written(100)).toBe("history");
    link(CLOSED);
    link(OPEN);
    await answer(1, 100);
    expect(boundary.written(100)).toBe("live");
  });

  it("ends with the follow and stops listening", async () => {
    const { boundary, listeners, answer } = harness();
    await answer(0, 0);
    expect(boundary.written(3)).toBe("live");
    boundary.stream({ state: "ended" });
    expect(boundary.written(3)).toBe("history");
    boundary.stop();
    expect(listeners.size).toBe(0);
  });
});
