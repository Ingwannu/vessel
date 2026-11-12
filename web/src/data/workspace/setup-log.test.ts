import { describe, expect, it } from "vitest";

import {
  appendText,
  EMPTY_LOG,
  MAX_LINES,
  OutputDecoder,
  plain,
  shownLines,
} from "./setup-log";

describe("setup's output as lines", () => {
  it("splits lines and keeps the one being written", () => {
    const log = appendText(EMPTY_LOG, "added 12 packages\nfound 0 vuln");
    expect(log.lines).toEqual(["added 12 packages"]);
    expect(shownLines(log)).toEqual(["added 12 packages", "found 0 vuln"]);
  });

  it("drops colours and cursor moves, keeps tabs", () => {
    expect(plain("\u001b[1;32mok\u001b[0m\tdone\u001b[1A").clean).toBe(
      "ok\tdone",
    );
    expect(plain("\u001b]0;title\u0007after").clean).toBe("after");
    expect(plain("a\u0008b\u007f").clean).toBe("ab");
  });

  it("drops an escape with intermediates whole (tput sgr0's ESC ( B)", () => {
    const log = appendText(EMPTY_LOG, "\u001b[1mbold\u001b(B\u001b[m plain\n");
    expect(log.lines).toEqual(["bold plain"]);
    const cut = appendText(EMPTY_LOG, "a\u001b(");
    expect(cut.pending).toBe("\u001b(");
    expect(appendText(cut, "Bb\n").lines).toEqual(["ab"]);
  });

  it("erases the line from the cursor, as a progress bar's redraw does", () => {
    const bar = appendText(EMPTY_LOG, "Downloading 100%\r\u001b[KDone\n");
    expect(bar.lines).toEqual(["Done"]);
    const whole = appendText(EMPTY_LOG, "Building 3/9\r\u001b[2KBuilt\n");
    expect(whole.lines).toEqual(["Built"]);
    const tail = appendText(EMPTY_LOG, "abcdef\rab\u001b[0K\n");
    expect(tail.lines).toEqual(["ab"]);
    const gone = appendText(EMPTY_LOG, "spinner \\\r\u001b[2K");
    expect(shownLines(gone)).toEqual([]);
  });

  it("holds back an escape sequence cut by a page's end", () => {
    const first = appendText(EMPTY_LOG, "start \u001b[3");
    expect(shownLines(first)).toEqual(["start "]);
    expect(first.pending).toBe("\u001b[3");
    const second = appendText(first, "2mgreen\u001b[0m\n");
    expect(second.lines).toEqual(["start green"]);
    expect(second.pending).toBe("");
  });

  it("writes over a line at each carriage return, as a terminal does", () => {
    const log = appendText(EMPTY_LOG, "10%\r55%\r100% done\n");
    expect(log.lines).toEqual(["100% done"]);
    const short = appendText(EMPTY_LOG, "Downloading\rOk\n");
    expect(short.lines).toEqual(["Okwnloading"]);
    const windows = appendText(EMPTY_LOG, "one\r\ntwo\r\n");
    expect(windows.lines).toEqual(["one", "two"]);
  });

  it("keeps the last lines and says it cut", () => {
    const many = Array.from(
      { length: MAX_LINES + 5 },
      (_, index) => `line ${String(index)}`,
    ).join("\n");
    const log = appendText(EMPTY_LOG, `${many}\n`);
    expect(log.lines).toHaveLength(MAX_LINES);
    expect(log.lines[0]).toBe("line 5");
    expect(log.cut).toBe(true);
  });

  it("decodes a character split across two pages", () => {
    const decoder = new OutputDecoder();
    const bytes = new TextEncoder().encode("é");
    expect(decoder.text(bytes.slice(0, 1))).toBe("");
    expect(decoder.text(bytes.slice(1))).toBe("é");
  });
});
