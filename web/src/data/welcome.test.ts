import { describe, expect, it } from "vitest";

import type { FleetState } from "./fleet";
import { bookNode, connectedAs } from "./fixtures/machines";
import { HERE } from "./machines";
import { fixtureId } from "../testing/ids";

import { firstScreen, hereIsDown } from "./welcome";

function state(extra: Partial<FleetState>): FleetState {
  return {
    link: { kind: "open", version: "0.1.0" },
    book: null,
    bookError: null,
    statuses: {},
    harnesses: {},
    profiles: {},
    models: {},
    agents: {},
    sessions: {},
    errors: {},
    selected: null,
    ...extra,
  };
}

describe("the first screen", () => {
  const book = [bookNode(HERE, { default: true }), bookNode("box")];

  it("waits while the book is unread or a machine is first dialled", () => {
    expect(firstScreen(state({}))).toBe("wait");
    expect(
      firstScreen(
        state({
          book,
          statuses: { [fixtureId("NodeKey", "box")]: { state: "connecting" } },
        }),
      ),
    ).toBe("wait");
  });

  it("welcomes once no machine connected, and stays while one redials", () => {
    const down = state({
      book,
      statuses: {
        [HERE]: { state: "failed", message: "no node" },
        [fixtureId("NodeKey", "box")]: {
          state: "reconnecting",
          message: "refused",
          attempt: 1,
          retry_in_ms: 500,
        },
      },
    });
    expect(firstScreen(down)).toBe("welcome");
    expect(firstScreen(state({ book: [] }))).toBe("welcome");
    const redialling = state({
      book,
      statuses: {
        ...down.statuses,
        [fixtureId("NodeKey", "box")]: { state: "connecting" },
      },
    });
    expect(firstScreen(redialling, "welcome")).toBe("welcome");
  });

  it("is the board as soon as one machine connects", () => {
    expect(
      firstScreen(
        state({
          book,
          statuses: { [fixtureId("NodeKey", "box")]: connectedAs("box") },
        }),
        "welcome",
      ),
    ).toBe("board");
  });

  it("says how to start this machine's node only while it is down", () => {
    expect(
      hereIsDown(book, { [HERE]: { state: "failed", message: "x" } }),
    ).toBe(true);
    expect(hereIsDown(book, { [HERE]: connectedAs("here") })).toBe(false);
    expect(hereIsDown([bookNode("box")], {})).toBe(false);
  });
});
