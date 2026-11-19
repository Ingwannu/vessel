import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import { BACKGROUND_SLOTS, inBackground } from "./file-work";

/** Work that holds its slot for a few milliseconds, counting how many
 * are out at once. */
function counter(): {
  work: (fail: boolean) => () => Promise<string>;
  most: () => number;
} {
  let out = 0;
  let most = 0;
  return {
    work: (fail) => async () => {
      out += 1;
      most = Math.max(most, out);
      await new Promise((settle) => {
        setTimeout(settle, 5);
      });
      out -= 1;
      if (fail) throw new Error("busy");
      return "done";
    },
    most: () => most,
  };
}

describe("background file work (the node's four file work slots)", () => {
  it("keeps a node's background work below its slots, in order", async () => {
    const node = fixtureId("NodeKey", "line-order");
    const count = counter();
    const ended: number[] = [];
    await Promise.all(
      [0, 1, 2, 3, 4, 5].map((index) =>
        inBackground(node, count.work(false)).then(() => {
          ended.push(index);
        }),
      ),
    );
    expect(count.most()).toBe(BACKGROUND_SLOTS);
    expect(BACKGROUND_SLOTS).toBeLessThan(4);
    expect(ended).toEqual([0, 1, 2, 3, 4, 5]);
  });

  it("frees the slot of work that failed", async () => {
    const node = fixtureId("NodeKey", "line-failure");
    const count = counter();
    const results = await Promise.allSettled(
      [true, true, false, false, false].map((fail) =>
        inBackground(node, count.work(fail)),
      ),
    );
    expect(results.map((result) => result.status)).toEqual([
      "rejected",
      "rejected",
      "fulfilled",
      "fulfilled",
      "fulfilled",
    ]);
    expect(count.most()).toBe(BACKGROUND_SLOTS);
  });

  it("gives each node its own line", async () => {
    const count = counter();
    await Promise.all(
      ["line-a", "line-b"].flatMap((name) =>
        [0, 1, 2].map(() =>
          inBackground(fixtureId("NodeKey", name), count.work(false)),
        ),
      ),
    );
    expect(count.most()).toBe(BACKGROUND_SLOTS * 2);
  });
});
