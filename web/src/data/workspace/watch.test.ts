import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { ObservedFile } from "../../wire/proto";
import { changed, diskAfter, sameObserved } from "./watch";

const hash = (digit: string) => fixtureId("ContentHash", digit.repeat(64));
const ready = (digit: string, size = 3): ObservedFile => ({
  kind: "ready",
  revision: { hash: hash(digit), size },
});
const missing: ObservedFile = { kind: "missing" };

describe("the shown file's watch", () => {
  it("takes a heartbeat with the same state as no change", () => {
    expect(changed(ready("a"), ready("a"), hash("a"))).toBe(false);
  });

  it("reads again when the disk moved past the anchor", () => {
    expect(changed(ready("a"), ready("b"), hash("a"))).toBe(true);
  });

  it("takes the page's own save as no change", () => {
    expect(changed(ready("a"), ready("b"), hash("b"))).toBe(false);
  });

  it("compares the first answer with what is shown, when that is known", () => {
    expect(changed(null, ready("b"), hash("a"))).toBe(true);
    expect(changed(null, ready("a"), hash("a"))).toBe(false);
    expect(changed(null, ready("b"), null)).toBe(false);
    expect(changed(null, missing, hash("a"))).toBe(true);
  });

  it("reports a deletion once", () => {
    expect(changed(ready("a"), missing, hash("a"))).toBe(true);
    expect(changed(missing, missing, hash("a"))).toBe(false);
  });

  it("says the disk is gone, back, or moved on", () => {
    expect(diskAfter("same", ready("a"), missing, hash("a"))).toBe("deleted");
    expect(diskAfter("deleted", missing, missing, hash("a"))).toBe("deleted");
    // Put back as shown: the same disk again, though the anchor moved.
    expect(diskAfter("deleted", missing, ready("a"), hash("a"))).toBe("same");
    expect(diskAfter("deleted", missing, ready("b"), hash("a"))).toBe(
      "changed",
    );
    // A heartbeat keeps a change the view has not read yet.
    expect(diskAfter("changed", ready("b"), ready("b"), hash("a"))).toBe(
      "changed",
    );
    // The page's own save over a change shows the disk again.
    expect(diskAfter("changed", ready("b"), ready("c"), hash("c"))).toBe(
      "same",
    );
  });

  it("tells states apart by kind, hash and size", () => {
    expect(sameObserved(missing, missing)).toBe(true);
    expect(sameObserved(missing, ready("a"))).toBe(false);
    expect(sameObserved(ready("a", 3), ready("a", 4))).toBe(false);
  });
});
