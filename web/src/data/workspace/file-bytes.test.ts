import { describe, expect, it } from "vitest";

import { dataUrl, decodedPage } from "./file-bytes";

describe("a file's bytes as an image's address", () => {
  it("joins pages before encoding, whatever their padding", () => {
    // Two pages whose own base64 would each be padded.
    const first = decodedPage(btoa("ab"));
    const second = decodedPage(btoa("cde"));
    expect(dataUrl([first, second], "image/png")).toBe(
      `data:image/png;base64,${btoa("abcde")}`,
    );
  });

  it("encodes a page longer than one chunk", () => {
    const page = new Uint8Array(0x8000 * 2 + 3).fill(65);
    const url = dataUrl([page], "image/gif");
    expect(url).toBe(`data:image/gif;base64,${btoa("A".repeat(page.length))}`);
  });
});
