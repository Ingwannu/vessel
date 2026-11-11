import { describe, expect, it } from "vitest";

import type { BookNode } from "../wire/bridge";
import { reach } from "./nodes";

const box: BookNode = {
  key: "gpu-box",
  label: "gpu-box",
  address: "ws://127.0.0.1:7002/rpc",
  transport: "ws",
  default: false,
  id: null,
};

describe("how a node is reached", () => {
  it("names the route in the page's words", () => {
    expect(reach(box)).toBe("direct");
    expect(reach({ ...box, transport: "paired" })).toBe("relay");
  });
});
