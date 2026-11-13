import { describe, expect, test } from "vitest";

import type { AttachmentLimits } from "../wire/proto";
import {
  attachmentName,
  kindOf,
  looksLike,
  mimeOf,
  refusal,
  refusalLine,
  sizeLabel,
} from "./attach";

const MIB = 1024 * 1024;

/** The node's limits (track A's bounds). */
const LIMITS: AttachmentLimits = {
  image: 10 * MIB,
  audio: 10 * MIB,
  file: 50 * MIB,
  per_prompt: 16,
  inline_per_prompt: 25 * MIB,
  per_session: 100 * MIB,
  chunk: MIB,
};

const SEES = { steer: false, resume: true, image: true, audio: false };

describe("a file's type and kind, as the node reads them", () => {
  test("the four inline image types are images, audio is audio", () => {
    expect(kindOf("image/png")).toBe("image");
    expect(kindOf("image/webp")).toBe("image");
    expect(kindOf("image/svg+xml")).toBe("file");
    expect(kindOf("audio/mpeg")).toBe("audio");
    expect(kindOf("application/pdf")).toBe("file");
  });

  test("a browser's type is narrowed to the node's shape", () => {
    expect(mimeOf("Image/PNG")).toBe("image/png");
    expect(mimeOf("text/plain; charset=utf-8")).toBe("text/plain");
    expect(mimeOf("")).toBe("application/octet-stream");
    expect(mimeOf("not a type")).toBe("application/octet-stream");
  });
});

describe("a file's name, as the node accepts it", () => {
  test("one component, no control characters, no edge dots", () => {
    expect(attachmentName("a/b\\c.txt", "file")).toBe("a_b_c.txt");
    expect(attachmentName('what?: "a|b".md', "file")).toBe("what__ _a_b_.md");
    expect(attachmentName(".env", "file")).toBe("_.env");
    expect(attachmentName("notes. ", "file")).toBe("notes");
    expect(attachmentName("..", "file")).toBe("file");
    expect(attachmentName("line\nbreak.md", "file")).toBe("line_break.md");
    expect(attachmentName("con.txt", "file")).toBe("_con.txt");
    expect(attachmentName("COM1", "file")).toBe("_COM1");
    expect(attachmentName("console.log", "file")).toBe("console.log");
  });

  test("at most 255 bytes, cut on a character, the extension kept", () => {
    const name = attachmentName(`${"가".repeat(100)}.png`, "file");
    expect(new TextEncoder().encode(name).length).toBeLessThanOrEqual(255);
    expect(name).toBe(`${"가".repeat(83)}.png`);
  });

  test("an image's first bytes", () => {
    const png = new Uint8Array([
      0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
    ]);
    expect(looksLike("image/png", png)).toBe(true);
    expect(looksLike("image/jpeg", png)).toBe(false);
    expect(looksLike("text/plain", png)).toBe(true);
  });
});

describe("what is refused before anything is sent", () => {
  test("a kind the agent does not take", () => {
    expect(refusal({ kind: "audio", size: 10 }, [], SEES, LIMITS)).toEqual({
      kind: "agent",
      what: "audio",
    });
    // Not yet said: images wait for the agent's answer.
    expect(refusal({ kind: "image", size: 10 }, [], null, LIMITS)).toEqual({
      kind: "agent",
      what: "image",
    });
    // Files are taken by every agent.
    expect(refusal({ kind: "file", size: 10 }, [], null, LIMITS)).toBeNull();
  });

  test("the kind's size, the count and the inline total", () => {
    expect(
      refusal({ kind: "image", size: 11 * MIB }, [], SEES, LIMITS),
    ).toEqual({ kind: "too-large", limit: 10 * MIB });
    expect(refusal({ kind: "file", size: 0 }, [], SEES, LIMITS)).toEqual({
      kind: "empty",
    });
    const sixteen = Array.from({ length: 16 }, () => ({
      kind: "file" as const,
      size: 1,
    }));
    expect(refusal({ kind: "file", size: 1 }, sixteen, SEES, LIMITS)).toEqual({
      kind: "too-many",
      limit: 16,
    });
    const images = [
      { kind: "image" as const, size: 9 * MIB },
      { kind: "image" as const, size: 9 * MIB },
      { kind: "file" as const, size: 40 * MIB },
    ];
    expect(
      refusal({ kind: "image", size: 8 * MIB }, images, SEES, LIMITS),
    ).toEqual({ kind: "inline-total", limit: 25 * MIB });
    expect(
      refusal({ kind: "image", size: 7 * MIB }, images, SEES, LIMITS),
    ).toBeNull();
  });

  test("said plainly", () => {
    expect(refusalLine({ kind: "agent", what: "image" }, "Codex")).toBe(
      "Codex does not take images",
    );
    expect(refusalLine({ kind: "too-large", limit: 10 * MIB }, "x")).toBe(
      "Larger than 10 MB",
    );
    expect(sizeLabel(512)).toBe("512 B");
    expect(sizeLabel(3 * MIB + MIB / 5)).toBe("3.2 MB");
  });
});
