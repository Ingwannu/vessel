import { describe, expect, it, vi } from "vitest";

import { Bridge } from "../../bridge/client";
import { fixtureId } from "../../testing/ids";
import { Fleet } from "../fleet";
import { BACKGROUND_SLOTS } from "./file-work";

import { HOST_FOLDERS_PROTOCOL } from "../../wire/proto";
import { hasProjectIcons, iconAddress, projectIcon } from "./project-icons";

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

  it("asks a node's icons on its background line, and goes on past a refusal", async () => {
    // The node refuses file work beyond four slots at once (`busy`): five
    // projects asked together reach it at most `BACKGROUND_SLOTS` at once.
    const fleet = new Fleet(
      new Bridge(
        { kind: "bootstrap", key: "a".repeat(64) },
        "ws://127.0.0.1:1/api/ws",
      ),
    );
    let out = 0;
    let most = 0;
    const call = vi.spyOn(fleet.bridge, "call").mockImplementation(async () => {
      out += 1;
      most = Math.max(most, out);
      await new Promise((settle) => {
        setTimeout(settle, 5);
      });
      out -= 1;
      throw new Error("busy");
    });
    const node = fixtureId("NodeKey", "laptop");
    const answers = await Promise.all(
      ["one", "two", "three", "four", "five"].map((id) =>
        projectIcon(fleet, node, fixtureId("ProjectId", id)),
      ),
    );
    expect(answers).toEqual([null, null, null, null, null]);
    expect(call).toHaveBeenCalledTimes(5);
    expect(most).toBe(BACKGROUND_SLOTS);
  });
});
