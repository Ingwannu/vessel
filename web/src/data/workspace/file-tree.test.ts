import { describe, expect, test } from "vitest";

import type { DirEntry } from "../../wire/proto";
import {
  absolutePath,
  isWithin,
  nameProblem,
  renamedOpen,
  type TreeRow,
  withEdit,
  withoutWithin,
} from "./file-tree";

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

function entry(path: string, depth: number, kind: "file" | "dir"): TreeRow {
  const name = path.split("/").at(-1) ?? path;
  const listed: DirEntry = { name, kind };
  return {
    kind: "entry",
    key: path,
    path,
    entry: listed,
    depth,
    open: kind === "dir",
  };
}

describe("the Explorer's name row (Paseo's EntryNameInputRow)", () => {
  const rows = [
    entry("src", 0, "dir"),
    entry("src/a.ts", 1, "file"),
    entry("b.md", 0, "file"),
  ];

  test("a new entry's row is first in its folder, one level in", () => {
    const made = withEdit(rows, {
      kind: "create",
      parent: "src",
      entry: "file",
    });
    expect(made.map((row) => [row.kind, row.depth])).toEqual([
      ["entry", 0],
      ["edit", 1],
      ["entry", 1],
      ["entry", 0],
    ]);
    const atRoot = withEdit(rows, { kind: "create", parent: "", entry: "dir" });
    expect(atRoot[0]).toMatchObject({
      kind: "edit",
      depth: 0,
      entry: "dir",
      initial: "",
    });
  });

  test("a renamed entry's row takes its place, its name typed in", () => {
    const renamed = withEdit(rows, {
      kind: "rename",
      path: "src/a.ts",
      entry: "file",
      name: "a.ts",
    });
    expect(renamed[1]).toMatchObject({
      kind: "edit",
      depth: 1,
      initial: "a.ts",
    });
    expect(renamed).toHaveLength(3);
  });

  test("names Paseo refuses, in its words", () => {
    expect(nameProblem("  ")).toBe("Invalid name");
    expect(nameProblem("..")).toBe("Invalid name");
    expect(nameProblem("a/b")).toBe("Name cannot contain path separators");
    expect(nameProblem("a\\b")).toBe("Name cannot contain path separators");
    expect(nameProblem("notes.md")).toBeNull();
  });
});

describe("open folders after a rename or a delete", () => {
  const open = new Set(["src", "src/ui", "srcs", "docs"]);

  test("a folder's own path and what is inside it, not a name that starts the same", () => {
    expect(isWithin("src/ui", "src")).toBe(true);
    expect(isWithin("src", "src")).toBe(true);
    expect(isWithin("srcs", "src")).toBe(false);
  });

  test("renamed: the folder and those inside it follow", () => {
    expect([...renamedOpen(open, "src", "lib")].sort()).toEqual([
      "docs",
      "lib",
      "lib/ui",
      "srcs",
    ]);
  });

  test("deleted: the folder and those inside it go", () => {
    expect([...withoutWithin(open, "src")].sort()).toEqual(["docs", "srcs"]);
  });
});
