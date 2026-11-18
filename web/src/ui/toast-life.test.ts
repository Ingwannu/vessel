import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  dismissToast,
  ERROR_TOAST_LIFE_MS,
  holdToast,
  releaseToast,
  shownToast,
  TOAST_LIFE_MS,
  toast,
  toastCopied,
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

  it("lives Paseo's 2.2 s, an error 3.2 s", () => {
    expect([TOAST_LIFE_MS, ERROR_TOAST_LIFE_MS]).toEqual([2200, 3200]);
    toast("Renamed laptop to desk");
    vi.advanceTimersByTime(2199);
    expect(shownToast.get()?.variant).toBe("success");
    vi.advanceTimersByTime(1);
    expect(shownToast.get()).toBeNull();
    toast("The clipboard refused the text", "error");
    vi.advanceTimersByTime(3199);
    expect(shownToast.get()?.variant).toBe("error");
    vi.advanceTimersByTime(1);
    expect(shownToast.get()).toBeNull();
  });

  it("a null duration stays until its own dismissal", () => {
    const dismiss = toast("Reloading the agent", "default", {
      durationMs: null,
    });
    vi.advanceTimersByTime(60_000);
    expect(shownToast.get()?.text).toBe("Reloading the agent");
    dismiss();
    expect(shownToast.get()).toBeNull();
  });

  it("a copied toast is a success that says it was copied", () => {
    toastCopied("Copied the session id");
    expect(shownToast.get()).toMatchObject({
      variant: "success",
      copied: true,
    });
  });

  it("a new toast replaces the one up", () => {
    toastCopied("Copied the session id");
    toast("Renamed laptop to desk");
    expect(shownToast.get()).toMatchObject({
      text: "Renamed laptop to desk",
      copied: false,
    });
  });

  it("the new toast runs its own life, not the rest of the old one's", () => {
    toast("first", "error");
    vi.advanceTimersByTime(3000);
    toast("second");
    vi.advanceTimersByTime(TOAST_LIFE_MS - 1);
    expect(shownToast.get()?.text).toBe("second");
    vi.advanceTimersByTime(1);
    expect(shownToast.get()).toBeNull();
  });

  it("dismissing a replaced toast leaves the newer one and its life", () => {
    const first = toast("Reloading the agent", "default", { durationMs: null });
    toast("Reloaded the agent");
    first();
    expect(shownToast.get()?.text).toBe("Reloaded the agent");
    vi.advanceTimersByTime(TOAST_LIFE_MS);
    expect(shownToast.get()).toBeNull();
  });

  it("a resting pointer holds the life left; leaving runs the rest", () => {
    toast("Renamed laptop to desk");
    const id = shownId();
    vi.advanceTimersByTime(1000);
    holdToast(id);
    vi.advanceTimersByTime(10_000);
    expect(shownToast.get()?.id).toBe(id);
    releaseToast(id);
    vi.advanceTimersByTime(TOAST_LIFE_MS - 1001);
    expect(shownToast.get()?.id).toBe(id);
    vi.advanceTimersByTime(1);
    expect(shownToast.get()).toBeNull();
  });

  it("a hold on a replaced toast does not stop the new one", () => {
    toast("first");
    const first = shownId();
    toast("second");
    holdToast(first);
    vi.advanceTimersByTime(TOAST_LIFE_MS);
    expect(shownToast.get()).toBeNull();
  });

  it("a toast that stays until dismissed ignores hover", () => {
    toast("Uploading", "default", { durationMs: null });
    const id = shownId();
    holdToast(id);
    releaseToast(id);
    vi.advanceTimersByTime(60_000);
    expect(shownToast.get()?.id).toBe(id);
  });
});
