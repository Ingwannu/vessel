import { describe, expect, it } from "vitest";

import {
  HOST_FOLDERS_PROTOCOL,
  hasProjectIcons,
  iconAddress,
} from "./project-icons";

describe("project icons (ADR 0112)", () => {
  it("draws the node's bytes with the type the node decided", () => {
    expect(iconAddress({ type: "svg", bytes: "PHN2Zy8+" })).toBe(
      "data:image/svg+xml;base64,PHN2Zy8+",
    );
    expect(iconAddress({ type: "png", bytes: "iVBORw==" })).toBe(
      "data:image/png;base64,iVBORw==",
    );
    expect(iconAddress({ type: "ico", bytes: "AAABAA==" })).toBe(
      "data:image/x-icon;base64,AAABAA==",
    );
  });

  it("asks only a node speaking the protocol", () => {
    expect(hasProjectIcons(null)).toBe(false);
    expect(hasProjectIcons(HOST_FOLDERS_PROTOCOL - 1)).toBe(false);
    expect(hasProjectIcons(HOST_FOLDERS_PROTOCOL)).toBe(true);
  });
});
