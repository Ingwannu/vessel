import { describe, expect, it } from "vitest";

import type { PairingOffer } from "../wire/proto";
import { bookNode, claude, connectedAs } from "./fixtures/machines";
import {
  AGENTS_COMMAND,
  checklistState,
  expiresIn,
  offerRoutes,
  PAIR_COMMAND,
  pairingChecks,
  START_COMMAND,
} from "./pairing";

const offer: PairingOffer = {
  v: 1,
  node: "node-id",
  key: "key",
  relays: ["wss://relay.example/v1"],
  direct: ["ws://127.0.0.1:7077"],
  invite: "invite",
  expires: "2026-11-11T07:10:00Z",
};

describe("what a link says", () => {
  it("names its routes, direct ones first, as host and port", () => {
    expect(offerRoutes(offer)).toEqual([
      "Directly at 127.0.0.1:7077",
      "Through the relay at relay.example",
    ]);
  });

  it("says when it expires, and nothing once it has", () => {
    const now = Date.parse("2026-11-11T07:00:30Z");
    expect(expiresIn(offer.expires, now)).toBe("in 10 minutes");
    expect(expiresIn(offer.expires, Date.parse("2026-11-11T07:09:30Z"))).toBe(
      "in 1 minute",
    );
    expect(
      expiresIn(offer.expires, Date.parse("2026-11-11T07:10:00Z")),
    ).toBeNull();
  });
});

describe("the checklist after Connect", () => {
  const studio = bookNode("studio", { transport: "paired" });

  it("waits on each step in turn", () => {
    const checks = pairingChecks(
      { kind: "pairing" },
      undefined,
      undefined,
      undefined,
    );
    expect(checks.map((check) => check.state)).toEqual([
      "pending",
      "pending",
      "pending",
    ]);
    expect(checklistState(checks)).toBe("running");
    const paired = pairingChecks(
      { kind: "paired", node: studio },
      { state: "connecting" },
      undefined,
      undefined,
    );
    expect(paired.map((check) => check.state)).toEqual([
      "passed",
      "pending",
      "pending",
    ]);
    expect(paired[0]?.result).toBe("Added to your machines as studio");
  });

  it("passes when the machine connects and has agents", () => {
    const checks = pairingChecks(
      { kind: "paired", node: studio },
      connectedAs("studio"),
      [claude],
      undefined,
    );
    expect(checks.map((check) => check.state)).toEqual([
      "passed",
      "passed",
      "passed",
    ]);
    expect(checks[1]?.result).toBe("studio · Linux x86_64 · Vessel 0.1.0");
    expect(checks[2]?.result).toBe("Claude Code");
    expect(checklistState(checks)).toBe("done");
  });

  it("stops at a refused link, says to make another, and skips the rest", () => {
    const checks = pairingChecks(
      {
        kind: "refused",
        problem: {
          from: "op",
          reason: "failed",
          message: "the invitation was already used",
        },
      },
      undefined,
      undefined,
      undefined,
    );
    expect(checks.map((check) => check.state)).toEqual([
      "failed",
      "skipped",
      "skipped",
    ]);
    expect(checks[0]?.result).toBe("the invitation was already used");
    expect(checks[0]?.fix?.command).toBe(PAIR_COMMAND);
    expect(checklistState(checks)).toBe("stopped");
  });

  it("says a busy page is to wait, not to make a new link", () => {
    const checks = pairingChecks(
      {
        kind: "refused",
        problem: { from: "op", reason: "busy", message: "busy" },
      },
      undefined,
      undefined,
      undefined,
    );
    expect(checks[0]?.fix?.command).toBeUndefined();
    expect(checks[0]?.fix?.text).toMatch(/try again in a moment/);
  });

  it("offers the start command when the machine cannot be reached", () => {
    const checks = pairingChecks(
      { kind: "paired", node: studio },
      { state: "failed", message: "connection refused" },
      undefined,
      undefined,
    );
    expect(checks[1]).toMatchObject({
      state: "failed",
      result: "connection refused",
      fix: { command: START_COMMAND },
    });
    expect(checks[2]?.state).toBe("skipped");
  });

  it("fails the last step when no agent is installed", () => {
    const checks = pairingChecks(
      { kind: "paired", node: studio },
      connectedAs("studio"),
      [],
      undefined,
    );
    expect(checks[2]).toMatchObject({
      state: "failed",
      fix: { command: AGENTS_COMMAND },
    });
  });
});
