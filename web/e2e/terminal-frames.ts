// What a page types into terminals, read off its bridge socket: the text
// of each `session.send` or `terminal.send` call's data input, in order.

import type { Page } from "@playwright/test";

type Method = "session.send" | "terminal.send";

/** The text a frame the page sent puts into a terminal through `method`,
 * or `null` for any other frame. */
function typedText(frame: unknown, method: Method): string | null {
  if (typeof frame !== "object" || frame === null || !("call" in frame)) {
    return null;
  }
  const call = frame.call;
  if (
    typeof call !== "object" ||
    call === null ||
    !("method" in call) ||
    call.method !== method ||
    !("params" in call)
  ) {
    return null;
  }
  const params = call.params;
  if (typeof params !== "object" || params === null || !("input" in params)) {
    return null;
  }
  // `session.send` wraps a terminal's input (`{ type: "terminal", input }`,
  // data/calls.ts); `terminal.send` carries it as is.
  const outer = params.input;
  const input =
    typeof outer === "object" &&
    outer !== null &&
    "type" in outer &&
    outer.type === "terminal" &&
    "input" in outer
      ? outer.input
      : outer;
  if (
    typeof input !== "object" ||
    input === null ||
    !("kind" in input) ||
    input.kind !== "data" ||
    !("data" in input) ||
    typeof input.data !== "string"
  ) {
    return null;
  }
  return Buffer.from(input.data, "base64").toString("utf8");
}

/** Collect, from now on, the text `page` sends into terminals through
 * `method`; the returned list grows as frames go out. Call it before the
 * page opens its socket. */
export function watchTyped(page: Page, method: Method): string[] {
  const typed: string[] = [];
  page.on("websocket", (socket) => {
    socket.on("framesent", (event) => {
      const text =
        typeof event.payload === "string"
          ? event.payload
          : event.payload.toString("utf8");
      const sent = typedText(JSON.parse(text), method);
      if (sent !== null) {
        typed.push(sent);
      }
    });
  });
  return typed;
}

/** `text` with each ESC shown as `ESC`, so a test can match an answer's
 * bytes without a control character in its pattern. */
export function shown(text: string): string {
  return text.replaceAll("\x1b", "ESC");
}
