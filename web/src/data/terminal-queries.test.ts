import { describe, expect, it } from "vitest";

import type { IDisposable, IFunctionIdentifier, IParser } from "@xterm/xterm";

import {
  ATTRIBUTES_REPLY,
  answerQueries,
  colourReply,
  cursorCell,
  cursorReply,
  type Drawn,
  hasDuty,
  isPrimaryAttributes,
  OPERATING_REPLY,
  type QueryView,
  rgbOf,
  statusQuery,
} from "./terminal-queries";

type CsiHandler = (params: (number | number[])[]) => boolean | Promise<boolean>;
type OscHandler = (data: string) => boolean | Promise<boolean>;

/** A parser that keeps its handlers for the test to call, as xterm would
 * for a parsed sequence. */
function fakeParser() {
  const csi: { id: IFunctionIdentifier; handler: CsiHandler; on: boolean }[] =
    [];
  const osc: { code: number; handler: OscHandler; on: boolean }[] = [];
  const parser: Pick<IParser, "registerCsiHandler" | "registerOscHandler"> = {
    registerCsiHandler(id, handler): IDisposable {
      const entry = { id, handler, on: true };
      csi.push(entry);
      return {
        dispose() {
          entry.on = false;
        },
      };
    },
    registerOscHandler(code, handler): IDisposable {
      const entry = { code, handler, on: true };
      osc.push(entry);
      return {
        dispose() {
          entry.on = false;
        },
      };
    },
  };
  const same = (a: IFunctionIdentifier, b: IFunctionIdentifier) =>
    a.final === b.final &&
    (a.prefix ?? "") === (b.prefix ?? "") &&
    (a.intermediates ?? "") === (b.intermediates ?? "");
  return {
    parser,
    /** What the page's handler for `id` returned: `null` when it has none. */
    csi(id: IFunctionIdentifier, params: number[]): boolean | null {
      const entry = csi.find((one) => one.on && same(one.id, id));
      if (entry === undefined) {
        return null;
      }
      const handled = entry.handler(params);
      if (typeof handled !== "boolean") {
        throw new Error("the page's handlers are synchronous");
      }
      return handled;
    },
    osc(code: number, data: string): boolean | null {
      const entry = osc.find((one) => one.on && one.code === code);
      if (entry === undefined) {
        return null;
      }
      const handled = entry.handler(data);
      if (typeof handled !== "boolean") {
        throw new Error("the page's handlers are synchronous");
      }
      return handled;
    },
  };
}

const DRAWN: Drawn = {
  foreground: { r: 0xe6, g: 0xe6, b: 0xe6 },
  background: { r: 0x0b, g: 0x0b, b: 0x0b },
  cursor: { r: 0xff, g: 0x80, b: 0x00 },
};

/** A view on 80 columns with the cursor at row 3, column 7 (0-based
 * `cursor`), answering while `open.now` holds. */
function view(
  open: { now: boolean },
  drawn: Drawn = DRAWN,
  cursor = { row: 2, col: 6, cols: 80 },
) {
  const sent: string[] = [];
  const queryView: QueryView = {
    answers: () => open.now,
    cursor: () => cursor,
    drawn: () => drawn,
    send: (reply) => sent.push(reply),
  };
  return { sent, queryView };
}

describe("the answers' bytes", () => {
  it("are native's and Paseo's daemon's", () => {
    expect(ATTRIBUTES_REPLY).toBe("\x1b[?62;4;22c");
    expect(OPERATING_REPLY).toBe("\x1b[0n");
    expect(cursorReply(2, 6, false)).toBe("\x1b[3;7R");
    expect(cursorReply(2, 6, true)).toBe("\x1b[?3;7R");
    expect(colourReply(11, { r: 0x0b, g: 0x0b, b: 0x0b })).toBe(
      "\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\",
    );
    expect(colourReply(12, { r: 255, g: 128, b: 0 })).toBe(
      "\x1b]12;rgb:ffff/8080/0000\x1b\\",
    );
  });

  it("read the drawn colours from computed CSS", () => {
    expect(rgbOf("rgb(230, 230, 230)")).toEqual({ r: 230, g: 230, b: 230 });
    expect(rgbOf("rgba(11, 12, 13, 0.5)")).toEqual({ r: 11, g: 12, b: 13 });
    expect(rgbOf("#ffffff")).toBeNull();
    expect(rgbOf("rgb(256, 0, 0)")).toBeNull();
    expect(rgbOf("")).toBeNull();
  });
});

describe("a cursor report", () => {
  it("names the last column while a wrap is pending, as native", () => {
    expect(cursorCell({ row: 3, col: 10, cols: 10 })).toEqual({
      row: 3,
      col: 9,
    });
    expect(cursorCell({ row: 0, col: 4, cols: 10 })).toEqual({
      row: 0,
      col: 4,
    });
  });

  it("says column 10 of 10 after a write to the last column", () => {
    const parser = fakeParser();
    const { sent, queryView } = view({ now: true }, DRAWN, {
      row: 3,
      col: 10,
      cols: 10,
    });
    answerQueries(parser.parser, queryView);
    expect(parser.csi({ final: "n" }, [6])).toBe(true);
    expect(parser.csi({ prefix: "?", final: "n" }, [6])).toBe(true);
    // Native's test: `ESC[4;10R` on a 10-column grid.
    expect(sent).toEqual(["\x1b[4;10R", "\x1b[?4;10R"]);
  });
});

describe("the answered set", () => {
  it("is DA1 with no parameter or a single 0", () => {
    expect(isPrimaryAttributes([])).toBe(true);
    expect(isPrimaryAttributes([0])).toBe(true);
    expect(isPrimaryAttributes([1])).toBe(false);
    expect(isPrimaryAttributes([0, 0])).toBe(false);
  });

  it("is a status query with exactly one parameter, 5 or 6", () => {
    expect(statusQuery([5])).toBe("operating");
    expect(statusQuery([6])).toBe("cursor");
    expect(statusQuery([])).toBeNull();
    expect(statusQuery([6, 1])).toBeNull();
    expect(statusQuery([[6, 1]])).toBeNull();
    expect(statusQuery([15])).toBeNull();
  });
});

describe("the handlers", () => {
  it("answer each query of the set, live and with the duty", () => {
    const parser = fakeParser();
    const { sent, queryView } = view({ now: true });
    answerQueries(parser.parser, queryView);
    expect(parser.csi({ final: "c" }, [])).toBe(true);
    expect(parser.csi({ final: "n" }, [5])).toBe(true);
    expect(parser.csi({ final: "n" }, [6])).toBe(true);
    expect(parser.csi({ prefix: "?", final: "n" }, [6])).toBe(true);
    for (const code of [10, 11, 12]) {
      expect(parser.osc(code, "?")).toBe(true);
    }
    expect(sent).toEqual([
      "\x1b[?62;4;22c",
      "\x1b[0n",
      "\x1b[3;7R",
      "\x1b[?3;7R",
      "\x1b]10;rgb:e6e6/e6e6/e6e6\x1b\\",
      "\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\",
      "\x1b]12;rgb:ffff/8080/0000\x1b\\",
    ]);
  });

  it("swallow the set without answering while history parses or another view has the duty", () => {
    const parser = fakeParser();
    const { sent, queryView } = view({ now: false });
    answerQueries(parser.parser, queryView);
    expect(parser.csi({ final: "c" }, [0])).toBe(true);
    expect(parser.csi({ final: "n" }, [6])).toBe(true);
    expect(parser.csi({ prefix: "?", final: "n" }, [6])).toBe(true);
    expect(parser.osc(11, "?")).toBe(true);
    expect(sent).toEqual([]);
  });

  it("leave everything else to xterm, whose own replies never become input", () => {
    const parser = fakeParser();
    const { sent, queryView } = view({ now: true });
    answerQueries(parser.parser, queryView);
    // DA1 with another parameter, other status codes, an OSC colour set or
    // a spaced `?` (native refuses it too, ADR 0073).
    expect(parser.csi({ final: "c" }, [1])).toBe(false);
    expect(parser.csi({ final: "n" }, [15])).toBe(false);
    expect(parser.csi({ prefix: "?", final: "n" }, [15])).toBe(false);
    expect(parser.osc(10, "rgb:0000/0000/0000")).toBe(false);
    expect(parser.osc(10, " ?")).toBe(false);
    // DA2, DA3 and DECRQM have no page handler at all.
    expect(parser.csi({ prefix: ">", final: "c" }, [])).toBeNull();
    expect(parser.csi({ prefix: "=", final: "c" }, [])).toBeNull();
    expect(parser.csi({ intermediates: "$", final: "p" }, [1])).toBeNull();
    expect(sent).toEqual([]);
  });

  it("answer no colour that is not known", () => {
    const parser = fakeParser();
    const { sent, queryView } = view(
      { now: true },
      { foreground: null, background: null, cursor: null },
    );
    answerQueries(parser.parser, queryView);
    expect(parser.osc(10, "?")).toBe(true);
    expect(sent).toEqual([]);
  });

  it("are removed on dispose", () => {
    const parser = fakeParser();
    const { queryView } = view({ now: true });
    answerQueries(parser.parser, queryView).dispose();
    expect(parser.csi({ final: "c" }, [])).toBeNull();
    expect(parser.osc(10, "?")).toBeNull();
  });
});

describe("the duty", () => {
  it("is native's: focus and typed, else the view that opened it", () => {
    const cases: [Parameters<typeof hasDuty>[0], boolean][] = [
      [{ origin: "attached", focused: true, keyed: true }, true],
      [{ origin: "attached", focused: true, keyed: false }, false],
      [{ origin: "attached", focused: false, keyed: true }, false],
      [{ origin: "attached", focused: false, keyed: false }, false],
      [{ origin: "opened", focused: false, keyed: false }, true],
      [{ origin: "opened", focused: true, keyed: true }, true],
    ];
    for (const [input, answers] of cases) {
      expect(hasDuty(input)).toBe(answers);
    }
  });
});
