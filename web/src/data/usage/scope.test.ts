import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import { usageReport } from "../fixtures/usage";
import {
  copiedScope,
  MACHINE_USAGE,
  usageChoices,
  usageNames,
  usageScopeKey,
  usageTargetFor,
} from "./scope";

describe("historical usage scopes", () => {
  it("forgets a selected profile on both legs of a machine round trip", () => {
    const laptop = fixtureId("NodeKey", "laptop");
    const gpu = fixtureId("NodeKey", "gpu-box");
    const chosen = {
      node: laptop,
      scope: { kind: "agent", harness: "claude-code", profile: "work" },
    } as const;
    expect(usageTargetFor(chosen, laptop)).toBe(chosen);
    const away = usageTargetFor(chosen, gpu);
    expect(away).toEqual({ node: gpu, scope: MACHINE_USAGE });
    expect(usageTargetFor(away, gpu)).toBe(away);
    expect(usageTargetFor(away, laptop)).toEqual({
      node: laptop,
      scope: MACHINE_USAGE,
    });
  });

  it("offers all profiles for unnamed reports, never a current-default interpretation", () => {
    const choices = usageChoices(
      [
        { harness: "claude-code", profile: null },
        { harness: "claude-code", profile: "work" },
        { harness: "claude-code", profile: "work" },
      ],
      MACHINE_USAGE,
    );
    expect(choices.map(({ label, scope }) => ({ label, scope }))).toEqual([
      { label: "All agents", scope: { kind: "machine" } },
      {
        label: "Claude Code · All profiles",
        scope: { kind: "agent", harness: "claude-code", profile: null },
      },
      {
        label: "Claude Code · Profile: work",
        scope: { kind: "agent", harness: "claude-code", profile: "work" },
      },
    ]);
  });

  it("keeps agent and named-profile identities distinct from all-profile scope", () => {
    const choices = usageChoices(
      [
        { harness: "claude-code", profile: "machine" },
        { harness: { added: "claude-code" }, profile: "machine" },
      ],
      MACHINE_USAGE,
    );
    expect(new Set(choices.map((choice) => choice.value)).size).toBe(5);
    expect(
      usageScopeKey({ kind: "agent", harness: "claude-code", profile: null }),
    ).not.toBe(
      usageScopeKey({
        kind: "agent",
        harness: "claude-code",
        profile: "machine",
      }),
    );
  });

  it("retains a selected historical scope after a later report omits its name", () => {
    const scope = { kind: "agent", harness: "codex", profile: "old" } as const;
    expect(usageChoices([], scope)).toEqual([
      { value: "machine", label: "All agents", scope: MACHINE_USAGE },
      { value: usageScopeKey(scope), label: "Codex · Profile: old", scope },
    ]);
  });

  it("copies the added-agent scope before callers can reuse/mutate it", () => {
    const scope = {
      kind: "agent",
      harness: { added: "helper" },
      profile: "one",
    } as const;
    const harness: { added: string } = { added: "helper" };
    const mutable = { ...scope, harness, profile: "one" };
    const snapshot = copiedScope(mutable);
    mutable.harness.added = "other";
    mutable.profile = "two";
    expect(snapshot).toEqual({
      kind: "agent",
      harness: { added: "helper" },
      profile: "one",
    });
  });

  it("stores names and partial coverage only, without cached token/cost facts", () => {
    const report = usageReport();
    report.incomplete = true;
    report.agents = [
      { harness: { added: "helper" }, profile: null, totals: report.totals },
    ];
    const names = usageNames(fixtureId("NodeKey", "laptop"), report);
    expect(names).toEqual({
      node: "laptop",
      incomplete: true,
      groups: [{ harness: { added: "helper" }, profile: null }],
    });
    const agent = report.agents.at(0);
    if (agent !== undefined && typeof agent.harness !== "string")
      agent.harness.added = "changed";
    expect(names.groups).toEqual([
      { harness: { added: "helper" }, profile: null },
    ]);
  });
});
