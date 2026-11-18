import { describe, expect, test } from "vitest";

import { absolutePath } from "./file-tree";

describe("an entry's path on the node (the Explorer's Copy path)", () => {
  test("the folder joined, one separator", () => {
    expect(absolutePath("/home/me/repo", "src/main.rs")).toBe(
      "/home/me/repo/src/main.rs",
    );
    expect(absolutePath("/home/me/repo/", "src")).toBe("/home/me/repo/src");
    expect(absolutePath("/home/me/repo", "")).toBe("/home/me/repo");
    expect(absolutePath("/", "etc")).toBe("/etc");
  });

  test("a Windows folder keeps its backslashes", () => {
    expect(absolutePath("C:\\work\\repo", "src/main.rs")).toBe(
      "C:\\work\\repo\\src\\main.rs",
    );
  });
});
