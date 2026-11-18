import { describe, expect, it } from "vitest";

import { type FileActionKey, fileActions, spaceActions } from "./file-actions";

const ALL: ReadonlySet<FileActionKey> = new Set<FileActionKey>([
  "new-file",
  "new-folder",
  "collapse-folder",
  "copy-path",
  "copy-relative-path",
  "download",
  "add-to-chat",
  "rename",
  "duplicate",
  "delete",
]);

/** The rows as Paseo's menu draws them: a "—" for each hairline. */
function drawn(rows: ReturnType<typeof fileActions>): string[] {
  return rows.flatMap((row) =>
    row.separatorBefore ? ["—", row.label] : [row.label],
  );
}

describe("the Explorer's file actions (Paseo's file-actions-menu.tsx)", () => {
  it("a file: Paseo's groups and order, Delete red", () => {
    const rows = fileActions({ kind: "file", open: false }, ALL);
    expect(drawn(rows)).toEqual([
      "New file",
      "New folder",
      "—",
      "Copy path",
      "Copy relative path",
      "Download",
      "Add to chat",
      "—",
      "Rename",
      "Duplicate",
      "—",
      "Delete",
    ]);
    expect(rows.filter((row) => row.destructive).map((row) => row.key)).toEqual(
      ["delete"],
    );
  });

  it("an open folder offers Collapse; no Download or Add to chat", () => {
    const rows = fileActions({ kind: "dir", open: true }, ALL);
    expect(rows.map((row) => row.key)).toEqual([
      "new-file",
      "new-folder",
      "collapse-folder",
      "copy-path",
      "copy-relative-path",
      "rename",
      "duplicate",
      "delete",
    ]);
  });

  it("a closed folder has nothing to collapse", () => {
    const rows = fileActions({ kind: "dir", open: false }, ALL);
    expect(rows.map((row) => row.key)).not.toContain("collapse-folder");
  });

  it("only what the screen can do, and no hairline before the first row", () => {
    const can = new Set<FileActionKey>(["copy-path", "download", "delete"]);
    const rows = fileActions({ kind: "file", open: false }, can);
    expect(drawn(rows)).toEqual(["Copy path", "Download", "—", "Delete"]);
  });

  it("the empty space makes a file or a folder at the root, nothing else", () => {
    expect(drawn(spaceActions(ALL))).toEqual(["New file", "New folder"]);
  });
});
