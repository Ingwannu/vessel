import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  dismissToast,
  holdToast,
  releaseToast,
  shownToast,
  toast,
} from "./toast-life";

function shownId(): number {
  const shown = shownToast.get();
  if (shown === null) throw new Error("Expected a toast up");
  return shown.id;
}

describe("one toast at a time (Paseo's ToastHost)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    const shown = shownToast.get();
    if (shown !== null) dismissToast(shown.id);
    vi.useRealTimers();
  });

  it("a new toast replaces the one up", () => {
    toast("Copied the session id");
    toast("Renamed laptop to desk");
    expect(shownToast.get()?.text).toBe("Renamed laptop to desk");
  });

  it("the new toast runs its own life, not the rest of the old one's", () => {
    toast("first");
    vi.advanceTimersByTime(3000);
    toast("second");
    vi.advanceTimersByTime(2000);
    expect(shownToast.get()?.text).toBe("second");
    vi.advanceTimersByTime(1999);
    expect(shownToast.get()?.text).toBe("second");
    vi.advanceTimersByTime(1);
    expect(shownToast.get()).toBeNull();
  });

  it("dismissing a replaced toast leaves the newer one and its life", () => {
    toast("first");
    const first = shownId();
    toast("second");
    dismissToast(first);
    expect(shownToast.get()?.text).toBe("second");
    vi.advanceTimersByTime(4000);
    expect(shownToast.get()).toBeNull();
  });

  it("an error stays until dismissed, and a new toast replaces it", () => {
    toast("The clipboard refused the text", "error");
    vi.advanceTimersByTime(60_000);
    expect(shownToast.get()?.tone).toBe("error");
    toast("Copied the session id");
    expect(shownToast.get()?.tone).toBe("ok");
    vi.advanceTimersByTime(4000);
    expect(shownToast.get()).toBeNull();
  });

  it("a resting pointer holds the life left; leaving runs the rest", () => {
    toast("Copied the session id");
    const id = shownId();
    vi.advanceTimersByTime(1000);
    holdToast(id);
    vi.advanceTimersByTime(10_000);
    expect(shownToast.get()?.id).toBe(id);
    releaseToast(id);
    vi.advanceTimersByTime(2999);
    expect(shownToast.get()?.id).toBe(id);
    vi.advanceTimersByTime(1);
    expect(shownToast.get()).toBeNull();
  });

  it("a hold on a replaced toast does not stop the new one", () => {
    toast("first");
    const first = shownId();
    toast("second");
    holdToast(first);
    vi.advanceTimersByTime(4000);
    expect(shownToast.get()).toBeNull();
  });
});
