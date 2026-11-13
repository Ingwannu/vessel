import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  FileRead,
  FileVersion,
  WireError,
  WriteExpect,
} from "../../wire/proto";
import type { Outcome } from "../outcome";
import { saveGuard, saveText, type SaveCalls } from "./save";

const READ: FileVersion = {
  size: 25,
  modified: 1_762_934_400_123,
  hash: fixtureId("ContentHash", "a".repeat(64)),
};
const WRITTEN: FileVersion = {
  size: 31,
  modified: 1_762_934_460_456,
  hash: fixtureId("ContentHash", "c".repeat(64)),
};
const CHANGED: FileVersion = {
  size: 40,
  modified: 1_762_934_450_789,
  hash: fixtureId("ContentHash", "b".repeat(64)),
};

/** The node's refusal of a call, as the bridge hands it to the page. */
function refused<T>(code: WireError["code"], message: string): Outcome<T> {
  return {
    ok: false,
    problem: {
      from: "call",
      failure: { kind: "remote", error: { code, message } },
      message,
    },
  };
}

/** A node holding one file: a guarded write lands only when the content
 * is the one the guard names, as `file.write` does. */
function node(onDisk: { version: FileVersion; text: string }): {
  calls: SaveCalls;
  writes: WriteExpect[];
  disk: () => string;
} {
  const writes: WriteExpect[] = [];
  let disk = onDisk;
  return {
    writes,
    disk: () => disk.text,
    calls: {
      write: (expect) => {
        writes.push(expect);
        if (expect.kind === "content" && expect.hash !== disk.version.hash) {
          return Promise.resolve(
            refused<FileVersion>("conflict", "the file changed"),
          );
        }
        disk = { version: WRITTEN, text: "mine" };
        return Promise.resolve({ ok: true, value: WRITTEN });
      },
      read: () => {
        const read: FileRead = {
          version: disk.version,
          content: { kind: "text", text: disk.text },
        };
        return Promise.resolve({ ok: true, value: read });
      },
    },
  };
}

describe("a save's guard", () => {
  it("is the content's hash the edit started from", () => {
    expect(saveGuard(READ)).toEqual({ kind: "content", hash: READ.hash });
  });

  it("is anything only for the person's Overwrite", () => {
    expect(saveGuard(null)).toEqual({ kind: "any" });
  });

  it("is none when the read carried no hash", () => {
    expect(saveGuard({ ...READ, hash: null })).toBeNull();
  });
});

describe("saving an edit", () => {
  it("goes through over an untouched file, guarded by its hash", async () => {
    const machine = node({ version: READ, text: "theirs" });
    const saved = await saveText(machine.calls, "mine", READ);
    expect(saved).toEqual({ kind: "saved", version: WRITTEN });
    expect(machine.writes).toEqual([{ kind: "content", hash: READ.hash }]);
  });

  it("over a file changed meanwhile is a conflict showing both states, and writes nothing", async () => {
    const machine = node({ version: CHANGED, text: "an agent's change" });
    const saved = await saveText(machine.calls, "mine", READ);
    expect(saved).toEqual({
      kind: "conflict",
      disk: CHANGED,
      diskText: "an agent's change",
      mine: "mine",
    });
    // One guarded write, refused: never a second one with `any`.
    expect(machine.writes).toEqual([{ kind: "content", hash: READ.hash }]);
    expect(machine.disk()).toBe("an agent's change");
  });

  it("over a file gone meanwhile is a conflict with nothing on disk", async () => {
    const calls: SaveCalls = {
      write: () => Promise.resolve(refused("conflict", "nothing is there")),
      read: () => Promise.resolve(refused("not_found", "not found")),
    };
    expect(await saveText(calls, "mine", READ)).toEqual({
      kind: "conflict",
      disk: null,
      diskText: null,
      mine: "mine",
    });
  });

  it("overwrites only when the person asked", async () => {
    const machine = node({ version: CHANGED, text: "an agent's change" });
    const saved = await saveText(machine.calls, "mine", null);
    expect(saved.kind).toBe("saved");
    expect(machine.writes).toEqual([{ kind: "any" }]);
  });

  it("writes nothing when the read carried no hash to guard by", async () => {
    const machine = node({ version: READ, text: "theirs" });
    const saved = await saveText(machine.calls, "mine", {
      ...READ,
      hash: null,
    });
    expect(saved.kind).toBe("failed");
    expect(machine.writes).toEqual([]);
  });

  it("says another refusal as the node said it", async () => {
    const calls: SaveCalls = {
      write: () => Promise.resolve(refused("forbidden", "read only")),
      read: () => Promise.reject(new Error("not read")),
    };
    expect(await saveText(calls, "mine", READ)).toEqual({
      kind: "failed",
      message: "read only",
    });
  });
});
