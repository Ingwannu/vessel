import { describe, expect, it } from "vitest";

import {
  buildDirectoryOptions,
  buildWorkingDirectorySuggestions,
  isOpenableProjectPath,
  shortenPath,
} from "./paths";

describe("Add project's directory rows (Paseo's picker options)", () => {
  const recentPaths = ["/home/me/code/shop", "/home/me/notes", "/srv/app"];

  it("lists every recent root for an empty field", () => {
    expect(
      buildWorkingDirectorySuggestions({
        recentPaths: [...recentPaths, "/srv/app", " "],
        machinePaths: ["/home/me/other"],
        query: "",
      }),
    ).toEqual(recentPaths);
  });

  it("keeps the matching recent roots, then the machine's answer, once each", () => {
    expect(
      buildWorkingDirectorySuggestions({
        recentPaths,
        machinePaths: ["/home/me/code/shop", "/home/me/code/shop-api"],
        query: "shop",
      }),
    ).toEqual(["/home/me/code/shop", "/home/me/code/shop-api"]);
    // `~` keeps every recent root.
    expect(
      buildWorkingDirectorySuggestions({
        recentPaths,
        machinePaths: [],
        query: "~/",
      }),
    ).toEqual(recentPaths);
  });

  it("puts a typed openable path first unless it is suggested", () => {
    expect(
      buildDirectoryOptions({
        recentPaths,
        machinePaths: [],
        query: "/opt/new",
      }),
    ).toEqual([{ kind: "path", path: "/opt/new" }]);
    expect(
      buildDirectoryOptions({
        recentPaths,
        machinePaths: [],
        query: "/srv/app",
      }),
    ).toEqual([{ kind: "suggestion", path: "/srv/app" }]);
    expect(
      buildDirectoryOptions({ recentPaths, machinePaths: [], query: "notes" }),
    ).toEqual([{ kind: "suggestion", path: "/home/me/notes" }]);
  });

  it("knows an openable path and a home", () => {
    for (const path of ["/a", "~", "~/a", "C:\\a", "c:/a", "\\\\server\\s"]) {
      expect(isOpenableProjectPath(path)).toBe(true);
    }
    expect(isOpenableProjectPath("code/shop")).toBe(false);
    expect(shortenPath("/home/me/code")).toBe("~/code");
    expect(shortenPath("/Users/me")).toBe("~");
    expect(shortenPath("/srv/app")).toBe("/srv/app");
  });
});
