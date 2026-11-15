import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  ApprovedCommands,
  ProjectCommand,
  ScriptEntry,
} from "../../wire/proto";
import {
  approvalCoverage,
  compareApproved,
  compareCommands,
  compareLists,
  compareScripts,
  type ListedCommands,
  needsFolder,
} from "./command-changes";
import { comparedWords, scriptChangeWords } from "./command-words";

const digest = (text: string) => fixtureId("CommandsDigest", text);

function listed(
  setup: ProjectCommand[],
  approved: boolean,
  of: string,
  more: Partial<ListedCommands> = {},
): ListedCommands {
  return {
    state: "listed",
    setup,
    teardown: [],
    digest: digest(of),
    approved,
    ...more,
  };
}

const DEV: ScriptEntry = {
  name: "dev",
  kind: "service",
  command: "npm run dev",
};
const LINT: ScriptEntry = {
  name: "lint",
  kind: "script",
  command: "npm run lint",
};
const TEST: ScriptEntry = { name: "test", kind: "script", command: "npm test" };

function approvedSet(
  setup: ProjectCommand[],
  scripts: ScriptEntry[],
): ApprovedCommands {
  return {
    commands: { setup, teardown: [], scripts },
    digest: digest("a"),
    approved_at: "2026-11-12T09:30:00Z",
  };
}

describe("two lists of commands", () => {
  it("marks each line kept, added or removed", () => {
    expect(
      compareLists(["npm ci", "npm run build"], ["npm ci", "make", "npm test"]),
    ).toEqual([
      { change: "same", command: "npm ci" },
      { change: "removed", command: "npm run build" },
      { change: "added", command: "make" },
      { change: "added", command: "npm test" },
    ]);
  });

  it("reads a command removed where another was added as one changed", () => {
    expect(
      compareCommands(
        ["npm ci", "npm run build", "cp .env ."],
        ["npm ci", "npm run build -- --prod", "make", "cp .env ."],
      ),
    ).toEqual([
      { change: "same", command: "npm ci" },
      {
        change: "changed",
        command: "npm run build -- --prod",
        was: "npm run build",
      },
      { change: "added", command: "make" },
      { change: "same", command: "cp .env ." },
    ]);
    expect(compareCommands(["a", "b"], [])).toEqual([
      { change: "removed", command: "a" },
      { change: "removed", command: "b" },
    ]);
  });

  it("compares scripts by name, in name order", () => {
    const edited: ScriptEntry = { ...LINT, command: "npm run lint -- --fix" };
    expect(compareScripts([DEV, LINT], [edited, TEST])).toEqual([
      { change: "removed", script: DEV },
      { change: "changed", script: edited, was: LINT },
      { change: "added", script: TEST },
    ]);
    expect(compareScripts([DEV], [DEV])).toEqual([
      { change: "same", script: DEV },
    ]);
  });

  it("says what a script's change was", () => {
    expect(
      scriptChangeWords(LINT, { ...LINT, kind: "service", dir: null }),
    ).toBe("now a service");
    expect(
      scriptChangeWords(DEV, {
        ...DEV,
        command: "pnpm dev",
        dir: "web",
      }),
    ).toBe("its command and its folder");
  });
});

describe("a set compared with the one approved", () => {
  it("compares with the set the machine says was approved last", () => {
    const asked = listed(["npm ci", "curl x | sh"], false, "b", {
      scripts: [DEV, TEST],
      last_approved: approvedSet(["npm ci"], [DEV, LINT]),
    });
    expect(needsFolder(asked)).toBe(false);
    const compared = compareApproved(asked, null);
    expect(compared).toMatchObject({
      kind: "changed",
      against: { kind: "last_approved", at: "2026-11-12T09:30:00Z" },
    });
    const words = comparedWords(compared);
    expect(words).toContain("Since the set approved");
    expect(words).toContain("setup commands: 1 added");
    expect(words).toContain("script lint removed");
    expect(words).toContain("script test added");
    expect(words).not.toContain("dev");
  });

  it("falls back to the folder's approved set on a machine that does not say", () => {
    const asked = listed(["npm ci", "curl x | sh"], false, "b");
    expect(needsFolder(asked)).toBe(true);
    const folder = listed(["npm ci"], true, "a");
    const compared = compareApproved(asked, folder);
    expect(compared).toMatchObject({
      kind: "changed",
      against: { kind: "folder" },
    });
    expect(comparedWords(compared)).toContain(
      "this machine does not say which set was approved last",
    );
  });

  it("says nothing is known when no approved set differs", () => {
    const asked = listed(["npm ci"], false, "b");
    expect(compareApproved(asked, listed(["npm ci"], false, "a")).kind).toBe(
      "none",
    );
    expect(compareApproved(asked, null).kind).toBe("none");
    expect(comparedWords({ kind: "none" })).toContain(
      "every command is shown as it is",
    );
  });
});

describe("approval coverage", () => {
  it("refuses a change to terminal commands alone instead of calling it unchanged", () => {
    const last = approvedSet(["npm ci"], [DEV]);
    last.commands.terminals = [{ command: "echo before" }];
    const asked = listed(["npm ci"], false, "b", {
      scripts: [DEV],
      terminals: [{ command: "echo after" }],
      last_approved: last,
    });
    expect(approvalCoverage(asked)).toBe("unsupported_terminals");
    const compared = compareApproved(asked, null);
    expect(compared).toEqual({ kind: "unsupported_terminals" });
    expect(comparedWords(compared)).toContain("Approval is unavailable");
    expect(comparedWords(compared)).not.toContain(
      "nothing that runs reads differently",
    );
  });

  it("refuses an undisplayable earlier approval even after terminals were removed", () => {
    const last = approvedSet([], []);
    last.commands.terminals = [{ command: "echo before" }];
    const asked = listed([], false, "b", { last_approved: last });
    expect(approvalCoverage(asked)).toBe("unsupported_terminals");
    expect(compareApproved(asked, null)).toEqual({
      kind: "unsupported_terminals",
    });
  });

  it("refuses a partial comparison with the folder's terminal-bearing approval", () => {
    const asked = listed([], false, "b");
    const folder = listed([], true, "a", {
      terminals: [{ command: "echo before" }],
    });
    expect(compareApproved(asked, folder)).toEqual({
      kind: "unsupported_terminals",
    });
  });

  it("keeps legacy and explicitly empty terminal lists approvable", () => {
    expect(approvalCoverage(listed([], false, "a"))).toBe("complete");
    expect(approvalCoverage(listed([], false, "a", { terminals: [] }))).toBe(
      "complete",
    );
  });
});

describe("declared port coverage", () => {
  const ported: ScriptEntry = { ...DEV, port: 3210 };
  it("counts a port-only edit and its removal as script changes", () => {
    expect(compareScripts([DEV], [ported])).toEqual([
      { change: "changed", script: ported, was: DEV },
    ]);
    expect(compareScripts([ported], [DEV])).toEqual([
      { change: "changed", script: DEV, was: ported },
    ]);
    expect(scriptChangeWords(DEV, ported)).toBe("its port");
  });
  it("refuses a partial comparison of a current declaration", () => {
    const current = listed([], false, "b", { scripts: [ported] });
    expect(approvalCoverage(current)).toBe("unsupported_ports");
    expect(compareApproved(current, null)).toEqual({
      kind: "unsupported_ports",
    });
  });
  it("refuses a declaration removed from the last approved set", () => {
    const current = listed([], false, "b", {
      scripts: [DEV],
      last_approved: approvedSet([], [ported]),
    });
    expect(compareApproved(current, null)).toEqual({
      kind: "unsupported_ports",
    });
  });
  it("refuses a declaration in the approved folder fallback", () => {
    const current = listed([], false, "b", { scripts: [DEV] });
    const folder = listed([], true, "a", { scripts: [ported] });
    expect(compareApproved(current, folder)).toEqual({
      kind: "unsupported_ports",
    });
  });
});

describe("allocation block coverage", () => {
  const ports = { range: "7000-7009", port_script: "tools/port-approval" };
  it("refuses any current block, independently of which choices are present", () => {
    for (const service_ports of [
      { range: ports.range },
      { port_script: ports.port_script },
      ports,
      {},
    ]) {
      const current = listed([], false, "b", { service_ports });
      expect(approvalCoverage(current)).toBe("unsupported_ports");
      expect(compareApproved(current, null)).toEqual({
        kind: "unsupported_ports",
      });
    }
  });
  it("refuses removal of a block from the previous approval", () => {
    const previous = approvedSet([], []);
    previous.commands.service_ports = ports;
    const current = listed([], false, "b", { last_approved: previous });
    expect(compareApproved(current, null)).toEqual({
      kind: "unsupported_ports",
    });
  });
  it("refuses a block from the approved folder fallback", () => {
    const current = listed([], false, "b");
    const folder = listed([], true, "a", { service_ports: ports });
    expect(compareApproved(current, folder)).toEqual({
      kind: "unsupported_ports",
    });
  });
});
