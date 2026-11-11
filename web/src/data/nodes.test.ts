import { describe, expect, it } from "vitest";

import type { BookNode } from "../wire/bridge";
import { machineMeant } from "./nodes";

const box: BookNode = {
  key: "gpu-box",
  label: "gpu-box",
  address: "ws://127.0.0.1:7002/rpc",
  transport: "ws",
  default: false,
  id: null,
};

describe("the machine meant", () => {
  const book = [
    box,
    { ...box, key: "laptop", default: true },
    { ...box, key: "nas" },
  ];

  it("is the one asked for, else the last chosen, else the default, else the first", () => {
    expect(machineMeant(book, ["nas", "gpu-box"])).toBe("nas");
    expect(machineMeant(book, [undefined, "gpu-box"])).toBe("gpu-box");
    expect(machineMeant(book, [undefined, null])).toBe("laptop");
    expect(machineMeant([box], [])).toBe("gpu-box");
    expect(machineMeant([], [])).toBeNull();
  });

  it("is only one that is accepted (a new session needs a connected one)", () => {
    const connected = (key: string) => key === "nas";
    expect(machineMeant(book, ["gpu-box", "laptop"], connected)).toBe("nas");
    expect(machineMeant(book, [], () => false)).toBeNull();
  });
});
