import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  ApprovedCommands,
  ProjectCommand,
  ScriptEntry,
} from "../../wire/proto";
import {
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
