import { afterEach, describe, expect, it } from "vitest";

import { fixtureId } from "../testing/ids";
import {
  forgetOpening,
  OPENING_KEY,
  openings,
  returnedDraft,
  settleOpening,
  startOpening,
  withOpening,
} from "./openings";
import { EMPTY, type Entry, type Timeline } from "./timeline-model";

const SESSION = fixtureId("SessionId", "s-1");
const AT = "2026-11-13T06:00:00Z";

function prompt(text: string, replayed = false): Entry {
  return {
    key: "p-1",
    seq: 4,
    at: AT,
    kind: "prompt",
    text,
    replayed,
    turn: 1,
    attachments: [],
  };
}

const STARTING: Timeline = {
  ...EMPTY,
  entries: [
    {
      key: "s-0",
      seq: 0,
      at: AT,
      kind: "status",
      status: { state: "starting" },
    },
  ],
  next: 1,
};

afterEach(() => {
  openings.set(new Map());
});

describe("a new session's first prompt", () => {
  it("is drawn at the end of the log, marked as sending, while the log has none", () => {
    startOpening(SESSION, "Fix the overview", AT);
    const shown = withOpening(STARTING, openings.get().get(SESSION));
    expect(shown.entries).toHaveLength(2);
    expect(shown.entries.at(-1)).toMatchObject({
      key: OPENING_KEY,
      kind: "prompt",
      text: "Fix the overview",
      turn: 1,
      seq: 1,
      sending: true,
    });
  });

  it("stays drawn once taken, until the log has the prompt", () => {
    startOpening(SESSION, "Fix the overview", AT);
    settleOpening(SESSION, { kind: "sent" });
    const opening = openings.get().get(SESSION);
    expect(withOpening(STARTING, opening).entries).toHaveLength(2);
    const logged: Timeline = {
      ...STARTING,
      entries: [...STARTING.entries, prompt("Fix the overview")],
    };
    expect(withOpening(logged, opening)).toBe(logged);
  });

  it("is not drawn once the node refused it", () => {
    startOpening(SESSION, "Fix the overview", AT);
    settleOpening(SESSION, { kind: "refused", message: "no" });
    expect(withOpening(STARTING, openings.get().get(SESSION))).toBe(STARTING);
  });

  it("does not count a replayed prompt as the log's own", () => {
    startOpening(SESSION, "Fix the overview", AT);
    const replayed: Timeline = {
      ...STARTING,
      entries: [...STARTING.entries, prompt("older", true)],
    };
    expect(
      withOpening(replayed, openings.get().get(SESSION)).entries,
    ).toHaveLength(3);
  });

  it("keeps the newest sixteen, and forgets on asking", () => {
    for (let index = 0; index < 20; index += 1) {
      startOpening(fixtureId("SessionId", `s-${String(index)}`), "x", AT);
    }
    expect(openings.get().size).toBe(16);
    expect(openings.get().has(fixtureId("SessionId", "s-0"))).toBe(false);
    forgetOpening(fixtureId("SessionId", "s-19"));
    expect(openings.get().has(fixtureId("SessionId", "s-19"))).toBe(false);
  });

  it("leaves a session it does not keep as it is", () => {
    const before = openings.get();
    settleOpening(SESSION, { kind: "sent" });
    forgetOpening(SESSION);
    expect(openings.get()).toBe(before);
    expect(withOpening(STARTING, undefined)).toBe(STARTING);
  });
});

describe("a refused first prompt", () => {
  it("goes back in the box, before what was typed there meanwhile", () => {
    expect(returnedDraft("Fix the overview", "")).toBe("Fix the overview");
    expect(returnedDraft("Fix the overview", "  ")).toBe("Fix the overview");
    expect(returnedDraft("Fix the overview", "and the tests")).toBe(
      "Fix the overview\n\nand the tests",
    );
  });
});
