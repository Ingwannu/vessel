import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import {
  quotaModalHost,
  quotaSummaryHost,
  type QuotaHostChoice,
} from "./hosts";

const FIRST = fixtureId("NodeKey", "first");
const ACTIVE = fixtureId("NodeKey", "active");
const PICKED = fixtureId("NodeKey", "picked");

function choice(): QuotaHostChoice {
  return {
    picked: PICKED,
    active: ACTIVE,
    hosts: [
      { key: FIRST, kind: "supported" },
      { key: ACTIVE, kind: "supported" },
      { key: PICKED, kind: "supported" },
    ],
  };
}

describe("account usage host choice", () => {
  it("uses the connected supported pick for sidebar and modal", () => {
    expect(quotaSummaryHost(choice())).toBe(PICKED);
    expect(quotaModalHost(choice())).toBe(PICKED);
  });

  it("falls back through active then first without keeping an offline pick", () => {
    const current = choice();
    const offline: QuotaHostChoice = {
      ...current,
      hosts: current.hosts.map((host) =>
        host.key === PICKED ? { ...host, kind: "offline" } : host,
      ),
    };
    expect(quotaSummaryHost(offline)).toBe(ACTIVE);
    expect(quotaModalHost(offline)).toBe(ACTIVE);
    expect(quotaSummaryHost({ ...offline, active: null })).toBe(FIRST);
    expect(quotaModalHost({ ...offline, active: null })).toBe(FIRST);
  });

  it("preserves a connected unsupported pick only in the modal", () => {
    const current = choice();
    const unsupported: QuotaHostChoice = {
      ...current,
      hosts: current.hosts.map((host) =>
        host.key === PICKED ? { ...host, kind: "unsupported" } : host,
      ),
    };
    expect(quotaSummaryHost(unsupported)).toBe(ACTIVE);
    expect(quotaModalHost(unsupported)).toBe(PICKED);
  });

  it("can explain an unsupported host even when no host reports", () => {
    const current: QuotaHostChoice = {
      picked: null,
      active: null,
      hosts: [
        { key: FIRST, kind: "unsupported" },
        { key: ACTIVE, kind: "offline" },
      ],
    };
    expect(quotaSummaryHost(current)).toBeNull();
    expect(quotaModalHost(current)).toBe(FIRST);
  });

  it("shows no host when every entry is offline or removed", () => {
    const offline: QuotaHostChoice = {
      picked: PICKED,
      active: ACTIVE,
      hosts: [{ key: PICKED, kind: "offline" }],
    };
    expect(quotaSummaryHost(offline)).toBeNull();
    expect(quotaModalHost(offline)).toBeNull();
    expect(quotaModalHost({ ...offline, hosts: [] })).toBeNull();
  });

  it("re-evaluates availability and retains the explicit user pick on return", () => {
    const current = Object.freeze(choice());
    const moved: QuotaHostChoice = {
      ...current,
      hosts: [{ key: ACTIVE, kind: "supported" }],
    };
    expect(quotaModalHost(current)).toBe(PICKED);
    expect(quotaModalHost(moved)).toBe(ACTIVE);
    expect(quotaModalHost(current)).toBe(PICKED);
    expect(current.picked).toBe(PICKED);
    expect(current.hosts.map((host) => host.key)).toEqual([
      FIRST,
      ACTIVE,
      PICKED,
    ]);
  });
});
