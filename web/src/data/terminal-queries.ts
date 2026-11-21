// The terminal queries the page answers itself (ADR 0073's closed set,
// TRACKER 826): a program asking for the device's attributes, its status,
// where the cursor is, or which colours are drawn gets one answer, from
// the view that has the duty, and only for live output.
//
// [Decision Log]
// - Purpose: nvim, and crossterm's cursor query in TUIs, ask the terminal
//   at once and wait (or time out) for the answer. The page answered
//   nothing: `terminal-input.ts` forwards real keys, paste and IME only,
//   so xterm's own replies were dropped with the replay's.
// - Paseo 0.15.4 answers these in its daemon, one process from the PTY
//   (`packages/server/src/terminal/terminal.ts` 883-922), and its browser
//   xterm swallows them (`terminal-emulator-runtime.ts` 641-662). Vessel's
//   node keeps raw bytes and parses nothing, so a viewer answers, as the
//   native app does (ADR 0073, `view/src/terminal/reply/` and `duty.rs`).
// - Choice: xterm parser handlers for exactly the five queries ADR 0073
//   answers (DA1, DSR 5, DSR 6, `CSI ? 6 n`, OSC 10/11/12 `?`), building
//   the same bytes as native and Paseo's daemon. A handler swallows its
//   query, so xterm's own reply never forms, and sends the answer only
//   while the bytes are live and this view has the duty.
// - Rejected: forwarding xterm's own replies (onData without user
//   provenance). That set is xterm's, wider than ADR 0073's (DECRQSS, DA2,
//   window reports), and ADR 0017 keeps xterm output from becoming input.
//   A denylist over it would miss future sequences.
// - Cost: the answer crosses the bridge, so a program that asks and exits
//   at once may miss it (ADR 0073 states the same of native). Only this
//   view knows its duty; two views that both have focus and have typed
//   both answer.

import type { IDisposable, IParser } from "@xterm/xterm";

/** A colour as drawn, 8 bits a channel. */
export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** The colours the terminal is drawn in now; `null` where unknown, so
 * that slot's query is not answered (native does the same with no
 * palette). */
export interface Drawn {
  foreground: Rgb | null;
  background: Rgb | null;
  cursor: Rgb | null;
}

/** How this view came to the terminal (`duty.rs` `Origin`). */
export type Origin = "opened" | "attached";

/** ADR 0073's duty (`view/src/terminal/duty.rs` 66-77): a view that has
 * focus and has typed answers; until one does, the view that opened the
 * terminal. A view knows of no other viewer, so "none qualifies" holds,
 * as on native (`Others::NoneQualify`). */
export function hasDuty(view: {
  origin: Origin;
  focused: boolean;
  keyed: boolean;
}): boolean {
  return (view.focused && view.keyed) || view.origin === "opened";
}

/** The answer to DA1 (Paseo's daemon, terminal.ts 886). */
export const ATTRIBUTES_REPLY = "\x1b[?62;4;22c";

/** The answer to `CSI 5 n` (terminal.ts 896). */
export const OPERATING_REPLY = "\x1b[0n";

/** The answer to `CSI 6 n`, or with `privateForm` to `CSI ? 6 n`, for a cursor
 * at 0-based `row` and `col`. */
export function cursorReply(
  row: number,
  col: number,
  privateForm: boolean,
): string {
  const mark = privateForm ? "?" : "";
  return `\x1b[${mark}${String(row + 1)};${String(col + 1)}R`;
}

/** The answer to `OSC code ; ?`: each 8-bit channel doubled to 16 bits,
 * ended by ST (native's `reply/mod.rs` 154). */
export function colourReply(code: 10 | 11 | 12, colour: Rgb): string {
  const channel = (value: number) => {
    const hex = value.toString(16).padStart(2, "0");
    return `${hex}${hex}`;
  };
  return `\x1b]${String(code)};rgb:${channel(colour.r)}/${channel(colour.g)}/${channel(colour.b)}\x1b\\`;
}

/** A computed CSS colour (`rgb(r, g, b)` or `rgba(r, g, b, a)`), or `null`
 * for anything else. */
export function rgbOf(css: string): Rgb | null {
  const match =
    /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*(?:,[^)]*)?\)$/.exec(
      css.trim(),
    );
  if (match === null) {
    return null;
  }
  const [r, g, b] = [match[1], match[2], match[3]].map(Number);
  if (
    r === undefined ||
    g === undefined ||
    b === undefined ||
    [r, g, b].some((value) => value > 255)
  ) {
    return null;
  }
  return { r, g, b };
}

/** xterm's parameters for one sequence: a number, or a number with
 * sub-parameters. */
type Params = readonly (number | readonly number[])[];

/** DA1: no parameter, or a single 0 (native `decode.rs` 74-80; Paseo's
 * terminal.ts 885). */
export function isPrimaryAttributes(params: Params): boolean {
  return params.length === 0 || (params.length === 1 && params[0] === 0);
}

/** `CSI n` with exactly one parameter, 5 or 6 (terminal.ts 892; native
 * `decode.rs` 66-71). */
export function statusQuery(params: Params): "operating" | "cursor" | null {
  if (params.length !== 1) {
    return null;
  }
  if (params[0] === 5) {
    return "operating";
  }
  return params[0] === 6 ? "cursor" : null;
}

/** What the handlers ask of their view. */
export interface QueryView {
  /** Whether an answer may go out now: the bytes being parsed are live
   * and this view has the duty. */
  answers: () => boolean;
  /** The cursor, 0-based, in the screen being parsed. */
  cursor: () => { row: number; col: number };
  /** The colours drawn now. */
  drawn: () => Drawn;
  /** Send an answer, in order with the keys. */
  send: (reply: string) => void;
}

/** Register the handlers on `parser`; disposing removes them. Registered
 * after xterm's own, so they run first (xterm calls the newest handler
 * first, and a `true` stops the rest). */
export function answerQueries(
  parser: Pick<IParser, "registerCsiHandler" | "registerOscHandler">,
  view: QueryView,
): IDisposable {
  const reply = (text: string): true => {
    if (view.answers()) {
      view.send(text);
    }
    return true;
  };
  const handlers: IDisposable[] = [
    parser.registerCsiHandler({ final: "c" }, (params) =>
      isPrimaryAttributes(params) ? reply(ATTRIBUTES_REPLY) : false,
    ),
    parser.registerCsiHandler({ final: "n" }, (params) => {
      const query = statusQuery(params);
      if (query === null) {
        return false;
      }
      if (query === "operating") {
        return reply(OPERATING_REPLY);
      }
      const { row, col } = view.cursor();
      return reply(cursorReply(row, col, false));
    }),
    parser.registerCsiHandler({ prefix: "?", final: "n" }, (params) => {
      if (statusQuery(params) !== "cursor") {
        return false;
      }
      const { row, col } = view.cursor();
      return reply(cursorReply(row, col, true));
    }),
    ...([10, 11, 12] as const).map((code) =>
      // Exactly `?`, as native asks (`decode.rs` 217); Paseo trims it,
      // the refusing-side difference ADR 0073 states.
      parser.registerOscHandler(code, (data) => {
        if (data !== "?") {
          return false;
        }
        const drawn = view.drawn();
        const colour =
          code === 10
            ? drawn.foreground
            : code === 11
              ? drawn.background
              : drawn.cursor;
        if (colour === null) {
          return true;
        }
        return reply(colourReply(code, colour));
      }),
    ),
  ];
  return {
    dispose() {
      for (const handler of handlers) {
        handler.dispose();
      }
    },
  };
}
