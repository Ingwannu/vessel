import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { halted, pause, QUIET_MS, quiet } from "./pace";

describe("the pace of a long read", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("waits out the rest of the quiet time after a read that came back at once", async () => {
    let done = false;
    void quiet(performance.now(), null).then(() => {
      done = true;
    });
    await vi.advanceTimersByTimeAsync(QUIET_MS - 10);
    expect(done).toBe(false);
    await vi.advanceTimersByTimeAsync(20);
    expect(done).toBe(true);
  });

  it("does not wait after a read that waited", async () => {
    const asked = performance.now();
    await vi.advanceTimersByTimeAsync(QUIET_MS + 1);
    let done = false;
    void quiet(asked, null).then(() => {
      done = true;
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(done).toBe(true);
  });

  it("is cut short when the page stops following", async () => {
    const controller = new AbortController();
    let done = false;
    void pause(60_000, controller.signal).then(() => {
      done = true;
    });
    expect(halted(controller.signal)).toBe(false);
    controller.abort();
    await vi.advanceTimersByTimeAsync(0);
    expect(done).toBe(true);
    expect(halted(controller.signal)).toBe(true);
  });
});
