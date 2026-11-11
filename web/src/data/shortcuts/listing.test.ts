import { expect, test } from "vitest";

import { modifiedCount, shortcutRow, shortcutRows } from "./listing";

test("a row says its keys as the platform writes them", () => {
  expect(shortcutRow("toggle-inspector", {}, "mac").words).toEqual(["⇧⌘B"]);
  expect(shortcutRow("toggle-inspector", {}, "other").words).toEqual([
    "Ctrl+Shift+B",
  ]);
  expect(shortcutRow("jump-session", {}, "other")).toMatchObject({
    fixed: true,
    words: ["Alt+1–9"],
  });
});

test("search by name or keys; Modified shows the changed only", () => {
  expect(
    shortcutRows("General", {}, "other", "palette", "all").map(
      (row) => row.action,
    ),
  ).toEqual(["palette"]);
  expect(
    shortcutRows("Session", {}, "other", "ctrl+f", "all").map(
      (row) => row.action,
    ),
  ).toEqual(["find"]);
  const changed = { find: ["Mod+J"] };
  expect(
    shortcutRows("Session", changed, "other", "", "modified").map(
      (row) => row.action,
    ),
  ).toEqual(["find"]);
  expect(modifiedCount(changed)).toBe(1);
});

test("a clash kept in the file is shown on both rows", () => {
  const kept = { find: ["Mod+K"] };
  expect(shortcutRow("find", kept, "other").clashes).toEqual(["palette"]);
  expect(shortcutRow("palette", kept, "other").clashes).toEqual(["find"]);
});
