import { describe, expect, it } from "vitest";

import {
  callIdOf,
  HERE,
  isLaunchId,
  isNodeKey,
  isSessionId,
  nodeKeyOf,
  nonceOf,
  sessionIdOf,
  streamIdOf,
} from "./ids";

describe("the page's handles", () => {
  it("takes a machine's key as the book writes one, or this machine's", () => {
    expect(isNodeKey("laptop")).toBe(true);
    expect(isNodeKey("gpu-box.2_a")).toBe(true);
    expect(isNodeKey("_here")).toBe(true);
    expect(HERE).toBe("_here");
    expect(isNodeKey("a".repeat(64))).toBe(true);
    // A book name starts with a letter or digit, is ASCII, and short.
    for (const text of ["", "-box", "_there", "a".repeat(65), "box/1", "bōx"]) {
      expect(isNodeKey(text)).toBe(false);
    }
    expect(nodeKeyOf("laptop")).toBe("laptop");
    expect(nodeKeyOf("../settings")).toBeNull();
  });

  it("takes a session's id as a node mints one", () => {
    const id = "3f2a9c1e-7b4d-4e8a-9c0f-1a2b3c4d5e6f";
    expect(isSessionId(id)).toBe(true);
    expect(sessionIdOf(id)).toBe(id);
    for (const text of ["", "s1", id.toUpperCase(), `${id}x`, id.slice(1)]) {
      expect(isSessionId(text)).toBe(false);
    }
    expect(sessionIdOf("3f2a")).toBeNull();
  });

  it("takes the bridge's launch and nonces as 32 bytes of hex", () => {
    expect(isLaunchId("a".repeat(64))).toBe(true);
    expect(isLaunchId("A".repeat(64))).toBe(false);
    expect(isLaunchId("a".repeat(63))).toBe(false);
    expect(nonceOf("0".repeat(64))).toBe("0".repeat(64));
    expect(() => nonceOf("xyz")).toThrow();
  });

  it("counts calls and streams from 1, as safe integers", () => {
    expect(callIdOf(1)).toBe(1);
    expect(streamIdOf(2)).toBe(2);
    for (const value of [0, -1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
      expect(() => callIdOf(value)).toThrow();
      expect(() => streamIdOf(value)).toThrow();
    }
  });
});
