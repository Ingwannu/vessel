// Following a machine's plugins (node review R1, R3, notes 3 and 5): a
// failed poll is not the end, a quiet answer is paced, a change is read
// again, the baseline comes before the read, and a change lets the controls
// go only after the list is read again.

import { describe, expect, it } from "vitest";

import type { Outcome } from "./outcome";
import { followPlugins, settleChange } from "./plugins-follow";

const dropped: Outcome<number> = {
  ok: false,
  problem: { from: "op", reason: "failed", message: "disconnected" },
};

/** A scripted node: each poll takes the next answer; the run stops when
 * the script is spent. */
function scripted(answers: Outcome<number>[]) {
  const stop = new AbortController();
  const log: string[] = [];
  const follow = {
    poll: (after: number) => {
      log.push(`poll ${after === Number.MAX_SAFE_INTEGER ? "any" : after}`);
      const next = answers.shift();
      if (next === undefined) {
        stop.abort();
        return Promise.resolve(dropped);
      }
      return Promise.resolve(next);
    },
    reload: () => {
      log.push("reload");
      return Promise.resolve();
    },
    quietly: () => {
      log.push("quiet");
      return Promise.resolve();
    },
    retry: () => {
      log.push("retry");
      return Promise.resolve();
    },
    now: () => 0,
  };
  return { follow, stop, log };
}

const at = (value: number): Outcome<number> => ({ ok: true, value });

describe("following a machine's plugins", () => {
  it("takes the baseline, then reads, then waits from it", async () => {
    const { follow, stop, log } = scripted([at(4), at(4), at(5)]);
    await followPlugins(follow, stop.signal);
    expect(log).toEqual([
      "poll any",
      "reload",
      "poll 4",
      "quiet",
      "poll 4",
      "reload",
      "poll 5",
    ]);
  });

  it("starts over after a dropped poll instead of ending", async () => {
    const { follow, stop, log } = scripted([at(4), dropped, at(6), at(6)]);
    await followPlugins(follow, stop.signal);
    expect(log).toEqual([
      "poll any",
      "reload",
      "poll 4",
      "retry",
      // The reconnect: a fresh baseline and a fresh read, so a change
      // missed while the link was down shows.
      "poll any",
      "reload",
      "poll 6",
      "quiet",
      "poll 6",
    ]);
  });

  it("still reads the list when even the baseline fails", async () => {
    const { follow, stop, log } = scripted([dropped, at(2)]);
    await followPlugins(follow, stop.signal);
    expect(log.slice(0, 4)).toEqual([
      "poll any",
      "reload",
      "retry",
      "poll any",
    ]);
  });

  it("asks nothing once stopped", async () => {
    const { follow, stop, log } = scripted([at(1)]);
    stop.abort();
    await followPlugins(follow, stop.signal);
    expect(log).toEqual([]);
  });
});

describe("the end of a change", () => {
  it("lets the controls go only after the list is read again", async () => {
    const order: string[] = [];
    let answer: () => void = () => {
      throw new Error("not read yet");
    };
    const read = new Promise<void>((resolve) => {
      answer = resolve;
    });
    const settled = settleChange({
      say: () => order.push("say"),
      reload: () => {
        order.push("reload");
        return read;
      },
      release: () => order.push("release"),
    });
    await Promise.resolve();
    expect(order).toEqual(["say", "reload"]);
    answer();
    await settled;
    expect(order).toEqual(["say", "reload", "release"]);
  });
});
