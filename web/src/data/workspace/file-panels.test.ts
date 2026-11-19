import { describe, expect, it } from "vitest";

import {
  fileExtension,
  filePanelOf,
  hasPreview,
  imageType,
} from "./file-panels";

describe("the panel a file opens in (Paseo's catalog)", () => {
  it("reads the extension after the base name's last dot, in lower case", () => {
    expect(fileExtension("docs/README.MD")).toBe("md");
    expect(fileExtension("a.b/notes")).toBe("");
    expect(fileExtension("dir\\photo.Jpeg")).toBe("jpeg");
    expect(fileExtension(".md")).toBe("md");
  });

  it("opens Markdown, HTML and images in their panels, the rest as source", () => {
    expect(filePanelOf("docs/plan.markdown")).toBe("markdown");
    expect(filePanelOf("site/index.htm")).toBe("html");
    for (const name of [
      "a.png",
      "a.jpg",
      "a.jpeg",
      "a.gif",
      "a.webp",
      "a.svg",
    ]) {
      expect(filePanelOf(name)).toBe("image");
    }
    expect(filePanelOf("src/main.rs")).toBe("source");
    expect(filePanelOf("Makefile")).toBe("source");
  });

  it("gives Markdown and HTML a Preview, and images their media type", () => {
    expect(hasPreview("markdown")).toBe(true);
    expect(hasPreview("html")).toBe(true);
    expect(hasPreview("image")).toBe(false);
    expect(hasPreview("source")).toBe(false);
    expect(imageType("logo.SVG")).toBe("image/svg+xml");
    expect(imageType("photo.jpg")).toBe("image/jpeg");
    expect(imageType("main.rs")).toBeNull();
  });
});
