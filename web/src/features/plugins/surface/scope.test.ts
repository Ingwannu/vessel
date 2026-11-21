import { describe, expect, it } from "vitest";

import type { PluginControl, Surface } from "../../../wire/proto";
import { boundDocuments, everyControl, needsAdmin, toneOf } from "./scope";

const text = (id: string): PluginControl => ({
  id,
  content: { kind: "text", text: id },
});
const bound = (id: string, settings: string): PluginControl => ({
  id,
  content: {
    kind: "settings_switch",
    settings,
    key: "on",
    label: "On",
    hint: null,
  },
});
const opens: PluginControl = {
  id: "open",
  content: {
    kind: "button",
    button: {
      title: "Open",
      availability: "enabled",
      behavior: {
        kind: "action",
        action: { kind: "open_screen", screen: "main" },
      },
    },
  },
};
const popover = (controls: PluginControl[]): PluginControl => ({
  id: "more",
  content: {
    kind: "button",
    button: {
      title: "More",
      availability: "enabled",
      behavior: { kind: "popover", surface: { controls } },
    },
  },
});

describe("a declared surface", () => {
  it("is walked through its containers and popovers", () => {
    const surface: Surface = {
      controls: [
        text("a"),
        {
          id: "card",
          content: {
            kind: "card",
            surface: { controls: [bound("b", "display")] },
          },
        },
        popover([bound("c", "names"), bound("d", "display")]),
      ],
    };
    expect(everyControl(surface).map((control) => control.id)).toEqual([
      "a",
      "card",
      "b",
      "more",
      "c",
      "d",
    ]);
    expect(boundDocuments(surface)).toEqual(["display", "names"]);
    expect(needsAdmin(surface)).toBe(true);
  });

  it("needs no admin when it only shows text and opens screens", () => {
    expect(
      needsAdmin({ controls: [text("a"), opens, popover([text("b")])] }),
    ).toBe(false);
    expect(
      needsAdmin({ controls: [text("a"), popover([bound("b", "display")])] }),
    ).toBe(true);
  });

  it("maps declared tones to the kit's", () => {
    expect([
      toneOf("neutral"),
      toneOf("success"),
      toneOf("warning"),
      toneOf("danger"),
    ]).toEqual(["neutral", "ok", "warn", "danger"]);
  });
});
