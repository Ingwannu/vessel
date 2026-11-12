import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { FileVersion } from "../../wire/proto";
import { sameContent } from "./files";

const READ: FileVersion = {
  size: 25,
  // What the page holds of the node's nanoseconds: rounded by JSON.
  modified: 1_762_934_400_123_456_800,
  hash: fixtureId("ContentHash", "a".repeat(64)),
};

describe("sameContent", () => {
  it("holds for the same size and hash whatever the time", () => {
    expect(sameContent(READ, { ...READ, modified: READ.modified + 512 })).toBe(
      true,
    );
  });

  it("does not hold for another content or size", () => {
    expect(
      sameContent(READ, {
        ...READ,
        hash: fixtureId("ContentHash", "b".repeat(64)),
      }),
    ).toBe(false);
    expect(sameContent(READ, { ...READ, size: 26 })).toBe(false);
  });

  it("never holds without a hash to compare", () => {
    expect(sameContent({ ...READ, hash: null }, { ...READ, hash: null })).toBe(
      false,
    );
    expect(sameContent(READ, { ...READ, hash: null })).toBe(false);
  });
});
