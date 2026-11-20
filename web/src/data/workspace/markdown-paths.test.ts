import { describe, expect, it } from "vitest";

import { markdownTarget } from "./markdown-paths";

describe("a Markdown file's link or image target", () => {
  it("resolves a relative target from the Markdown file's folder", () => {
    expect(markdownTarget("./images/a.png", "docs/guide.md", null)).toEqual({
      kind: "file",
      path: "docs/images/a.png",
    });
    expect(markdownTarget("../logo.svg", "docs/guide.md", null)).toEqual({
      kind: "file",
      path: "logo.svg",
    });
    expect(markdownTarget("notes.md#install", "README.md", null)).toEqual({
      kind: "file",
      path: "notes.md",
    });
  });

  it("decodes an encoded name and ignores a query", () => {
    expect(markdownTarget("my%20shot.png?raw=1", "a/b.md", null)).toEqual({
      kind: "file",
      path: "a/my shot.png",
    });
  });

  it("keeps an absolute target inside the root, by the root's folder", () => {
    expect(
      markdownTarget("/work/app/assets/a.png", "README.md", "/work/app"),
    ).toEqual({ kind: "file", path: "assets/a.png" });
    expect(
      markdownTarget("file:///work/app/a.png", "README.md", "/work/app/"),
    ).toEqual({ kind: "file", path: "a.png" });
    expect(
      markdownTarget("/work/other/a.png", "README.md", "/work/app"),
    ).toEqual({ kind: "none" });
    expect(markdownTarget("/work/app/a.png", "README.md", null)).toEqual({
      kind: "none",
    });
  });

  it("refuses a target that leaves the root", () => {
    expect(markdownTarget("../../x.png", "docs/a.md", null)).toEqual({
      kind: "none",
    });
    expect(markdownTarget("~/x.png", "a.md", null)).toEqual({ kind: "none" });
  });

  it("hands web and data addresses back, and nothing else", () => {
    expect(markdownTarget("https://example.com/a.png", "a.md", null)).toEqual({
      kind: "web",
      url: "https://example.com/a.png",
    });
    expect(markdownTarget("data:image/png;base64,AA==", "a.md", null)).toEqual({
      kind: "data",
      url: "data:image/png;base64,AA==",
    });
    for (const target of ["#top", "", "mailto:a@example.com", "blob:x"]) {
      expect(markdownTarget(target, "a.md", null)).toEqual({ kind: "none" });
    }
  });
});
