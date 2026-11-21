import { describe, expect, test } from "vitest";

import type { NodeKey } from "../wire/bridge";
import type { PushFocus, ReportPushPresence } from "../wire/proto";
import { fixtureId } from "../testing/ids";
import {
  createPresenceTracker,
  HEARTBEAT_INTERVAL_MS,
  type PresenceTracker,
  refusesPresence,
  type ShownSession,
} from "./presence";

const BOX = fixtureId("NodeKey", "box");
const LAB = fixtureId("NodeKey", "lab");
const S1 = fixtureId("SessionId", "s1");
const S2 = fixtureId("SessionId", "s2");

/** The machine's rule, as vessel-node `push/policy.rs:13-55` ports Paseo's
 * `agent-attention-policy.ts:3,42-76`: an `active` report is stamped with
 * the machine's time; present means stamped within 180 s; a present
 * window focused on the target suppresses; else the most recently active
 * present window is told; else push. */
class Machine {
  readonly #windows = new Map<
    string,
    { focus: PushFocus; at: number | null }
  >();

  report(window: string, report: ReportPushPresence, now: number): void {
    const before = this.#windows.get(window);
    this.#windows.set(window, {
      focus: report.focus,
      at: report.activity === "active" ? now : (before?.at ?? null),
    });
  }

  /** `suppressed`, `push`, or the window told. */
  plan(target: PushFocus, now: number): string {
    let recipient: { window: string; at: number } | null = null;
    for (const [window, state] of this.#windows) {
      const at = state.at === null ? null : Math.min(state.at, now);
      if (at === null || now - at > 180_000) {
        continue;
      }
      if (JSON.stringify(state.focus) === JSON.stringify(target)) {
        return "suppressed";
      }
      if (recipient === null || at > recipient.at) {
        recipient = { window, at };
      }
    }
    return recipient === null ? "push" : recipient.window;
  }
}

/** A tracker on a fake clock, its reports kept and given to `machine`. */
function tab(
  options: {
    nodes?: NodeKey[];
    visible?: boolean;
    shown?: ShownSession | null;
    machine?: Machine;
    name?: string;
  } = {},
) {
  let now = 0;
  const nodes = options.nodes ?? [BOX];
  const sent: { node: NodeKey; at: number; report: ReportPushPresence }[] = [];
  const tracker: PresenceTracker = createPresenceTracker({
    link: {
      nodes: () => nodes,
      send(node, report) {
        sent.push({ node, at: now, report });
        options.machine?.report(options.name ?? "window", report, now);
      },
    },
    now: () => now,
    initialVisible: options.visible ?? true,
    initialShown: options.shown ?? null,
  });
  return {
    tracker,
    sent,
    nodes,
    /** Move the clock, running the 15 s heartbeat as the hook's timer does. */
    pass(ms: number) {
      const end = now + ms;
      let next =
        (Math.floor(now / HEARTBEAT_INTERVAL_MS) + 1) * HEARTBEAT_INTERVAL_MS;
      while (next <= end) {
        now = next;
        tracker.sendHeartbeat();
        next += HEARTBEAT_INTERVAL_MS;
      }
      now = end;
    },
    at: () => now,
    last: (node: NodeKey = BOX) =>
      sent.filter((each) => each.node === node).at(-1)?.report,
  };
}

describe("presence, as Paseo's client activity tracker", () => {
  test("the first heartbeat carries opening the page; later ones carry only new input", () => {
    const page = tab();
    page.tracker.sendHeartbeat();
    expect(page.last()).toEqual({
      focus: { focus: "elsewhere" },
      activity: "active",
    });
    page.pass(HEARTBEAT_INTERVAL_MS);
    expect(page.last()?.activity).toBe("unchanged");
    page.tracker.recordUserActivity();
    page.pass(HEARTBEAT_INTERVAL_MS);
    expect(page.last()?.activity).toBe("active");
    page.pass(HEARTBEAT_INTERVAL_MS);
    expect(page.last()?.activity).toBe("unchanged");
  });

  test("immediate heartbeats go at most every 5 s (client-activity-tracker.ts:2,67-73)", () => {
    const page = tab();
    page.tracker.maybeSendImmediateHeartbeat();
    page.tracker.maybeSendImmediateHeartbeat();
    expect(page.sent).toHaveLength(1);
    page.pass(4_999);
    page.tracker.maybeSendImmediateHeartbeat();
    expect(page.sent).toHaveLength(1);
    page.pass(1);
    page.tracker.maybeSendImmediateHeartbeat();
    expect(page.sent).toHaveLength(2);
  });

  test("a new shown session goes at once, as activity; the same one sends nothing (:78-89)", () => {
    const page = tab({ nodes: [BOX, LAB] });
    page.tracker.setShown({ node: BOX, id: S1 });
    expect(page.sent.map((each) => [each.node, each.report])).toEqual([
      [BOX, { focus: { focus: "session", session: S1 }, activity: "active" }],
      [LAB, { focus: { focus: "elsewhere" }, activity: "active" }],
    ]);
    page.tracker.setShown({ node: BOX, id: S1 });
    expect(page.sent).toHaveLength(2);
    page.pass(1_000);
    page.tracker.setShown(null);
    expect(page.last()).toEqual({
      focus: { focus: "elsewhere" },
      activity: "active",
    });
  });

  test("hidden reports background at the next heartbeat, not at once; visible again is input (:90-105)", () => {
    const page = tab({ shown: { node: BOX, id: S1 } });
    page.tracker.sendHeartbeat();
    page.pass(60_000);
    const before = page.sent.length;
    expect(page.tracker.notifyAppVisibility(false)).toEqual({ changed: true });
    expect(page.tracker.notifyAppVisibility(false)).toEqual({ changed: false });
    expect(page.sent).toHaveLength(before);
    page.pass(HEARTBEAT_INTERVAL_MS);
    expect(page.last()).toEqual({
      focus: { focus: "background" },
      activity: "unchanged",
    });
    page.tracker.notifyAppVisibility(true);
    page.tracker.maybeSendImmediateHeartbeat();
    expect(page.last()).toEqual({
      focus: { focus: "session", session: S1 },
      activity: "active",
    });
  });

  test("a machine that connects is told of input from the last 15 s only, then afresh after it went away", () => {
    const nodes = [BOX];
    const page = tab({ nodes });
    page.pass(16_000);
    nodes.push(LAB);
    page.tracker.sendTo(LAB);
    expect(page.last(LAB)?.activity).toBe("unchanged");
    page.tracker.recordUserActivity();
    page.pass(10_000);
    page.tracker.forget(LAB);
    page.tracker.sendTo(LAB);
    expect(page.last(LAB)?.activity).toBe("active");
    // Not connected: nothing goes to it.
    page.tracker.sendTo(fixtureId("NodeKey", "gone"));
    expect(
      page.sent.some((each) => each.node === fixtureId("NodeKey", "gone")),
    ).toBe(false);
  });
});

describe("what the machine makes of it (Paseo's 180 s policy)", () => {
  const other: PushFocus = { focus: "session", session: S2 };

  test("a window that goes stale lets push resume 180 s after its last input", () => {
    const machine = new Machine();
    const page = tab({ machine, shown: { node: BOX, id: S1 } });
    page.tracker.sendHeartbeat();
    expect(machine.plan(other, page.at())).toBe("window");
    // No more input: heartbeats keep coming, unchanged.
    page.pass(180_000);
    expect(machine.plan(other, page.at())).toBe("window");
    expect(machine.plan({ focus: "session", session: S1 }, page.at())).toBe(
      "suppressed",
    );
    page.pass(1);
    expect(machine.plan(other, page.at())).toBe("push");
    expect(machine.plan({ focus: "session", session: S1 }, page.at())).toBe(
      "push",
    );
    // A window that sends nothing at all (closed, or its machine gone) is the same.
    page.pass(600_000);
    expect(machine.plan(other, page.at())).toBe("push");
  });

  test("input that waited for a heartbeat keeps presence at most 15 s past Paseo's", () => {
    const machine = new Machine();
    const page = tab({ machine });
    page.tracker.sendHeartbeat();
    page.pass(1_000);
    page.tracker.recordUserActivity();
    page.tracker.maybeSendImmediateHeartbeat(); // goes: stamped at 1 s
    page.pass(2_000);
    page.tracker.recordUserActivity(); // at 3 s
    page.tracker.maybeSendImmediateHeartbeat(); // throttled: goes at 15 s
    expect(page.sent).toHaveLength(2);
    // Paseo's window would stop being present at 183 s; this one at 195 s.
    page.pass(195_000 - page.at());
    expect(machine.plan(other, page.at())).toBe("window");
    page.pass(1);
    expect(machine.plan(other, page.at())).toBe("push");
  });

  test("a hidden tab is present but never suppresses, as Paseo counts it", () => {
    const machine = new Machine();
    const page = tab({ machine, shown: { node: BOX, id: S1 } });
    const target: PushFocus = { focus: "session", session: S1 };
    page.tracker.sendHeartbeat();
    expect(machine.plan(target, page.at())).toBe("suppressed");
    page.tracker.notifyAppVisibility(false);
    page.pass(HEARTBEAT_INTERVAL_MS);
    // Hidden: the machine tells this window rather than suppressing.
    expect(machine.plan(target, page.at())).toBe("window");
    page.pass(180_000 - HEARTBEAT_INTERVAL_MS);
    expect(machine.plan(target, page.at())).toBe("window");
    page.pass(1);
    expect(machine.plan(target, page.at())).toBe("push");
  });

  test("of two windows the one with the latest input is told", () => {
    const machine = new Machine();
    const one = tab({ machine, name: "one" });
    const two = tab({ machine, name: "two" });
    one.tracker.sendHeartbeat();
    two.pass(10_000);
    two.tracker.recordUserActivity();
    two.tracker.sendHeartbeat();
    expect(machine.plan(other, 10_000)).toBe("two");
  });
});

test("only a lasting refusal stops presence to a machine", () => {
  const remote = (code: "forbidden" | "unsupported" | "unavailable") => ({
    from: "call" as const,
    failure: {
      kind: "remote" as const,
      error: { code, message: "no" },
    },
    message: "no",
  });
  expect(refusesPresence(remote("forbidden"))).toBe(true);
  expect(refusesPresence(remote("unsupported"))).toBe(true);
  expect(refusesPresence(remote("unavailable"))).toBe(false);
  expect(
    refusesPresence({
      from: "call",
      failure: {
        kind: "not_in_protocol",
        method: "push.presence",
        needs: 22,
        negotiated: 21,
      },
      message: "older",
    }),
  ).toBe(true);
  expect(
    refusesPresence({ from: "op", reason: "failed", message: "socket closed" }),
  ).toBe(false);
});
