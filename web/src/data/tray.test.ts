import { expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { AttachmentLimits, AttachmentRef } from "../wire/proto";
import { Tray } from "./tray";
import { trayReady } from "./tray-item";
import { handOver, trayOf } from "./trays";
import type { UploadCalls } from "./upload";

const MIB = 1024 * 1024;

const LIMITS: AttachmentLimits = {
  image: 10 * MIB,
  audio: 10 * MIB,
  file: 50 * MIB,
  per_prompt: 16,
  inline_per_prompt: 25 * MIB,
  per_session: 100 * MIB,
  chunk: MIB,
};

/** A node whose writes wait until `release` is called, counting uploads
 * open at once. */
function node() {
  let open = 0;
  let most = 0;
  let made = 0;
  const waiting: (() => void)[] = [];
  let didBegin = () => {};
  const begun = new Promise<void>((resolve) => {
    didBegin = resolve;
  });
  const calls: UploadCalls = {
    begin: () => {
      open += 1;
      didBegin();
      most = Math.max(most, open);
      made += 1;
      return Promise.resolve({
        ok: true,
        value: {
          upload: fixtureId("UploadId", `u${String(made)}`),
          chunk: MIB,
        },
      });
    },
    write: async (_upload, offset, data) => {
      await new Promise<void>((resolve) => waiting.push(resolve));
      return { ok: true, value: { received: offset + atob(data).length } };
    },
    finish: (upload) => {
      open -= 1;
      const ref: AttachmentRef = {
        id: fixtureId("AttachmentId", upload),
        name: "x",
        kind: "file",
        mime: "text/plain",
        size: 1,
      };
      return Promise.resolve({ ok: true, value: ref });
    },
  };
  // Lets every write that is waiting, or comes within a few turns of
  // the event loop, go.
  const release = async () => {
    for (let turn = 0; turn < 20; turn += 1) {
      waiting.shift()?.();
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  };
  return { calls, release, begun, most: () => most };
}

const file = (name: string, type = "text/plain") =>
  new File(["hello"], name, { type });

test("checked once there is a session, uploaded at Send, two at a time", async () => {
  const tray = new Tray();
  tray.add([file("a.txt"), file("b.txt"), file("c.txt")]);
  expect(tray.items.get().map((item) => item.state.kind)).toEqual([
    "held",
    "held",
    "held",
  ]);
  expect(tray.ready()).toEqual({ kind: "uploading" });
  const fake = node();
  tray.target({
    calls: fake.calls,
    abilities: null,
    limits: LIMITS,
    agent: "Codex",
  });
  // Checked, not sent: nothing is begun before Send.
  expect(tray.items.get().map((item) => item.state.kind)).toEqual([
    "attached",
    "attached",
    "attached",
  ]);
  expect(fake.most()).toBe(0);
  const uploaded = tray.upload();
  await fake.release();
  expect(fake.most()).toBe(2);
  const ready = await uploaded;
  expect(ready.kind).toBe("ready");
  expect(ready.kind === "ready" ? ready.refs : []).toHaveLength(3);
});

test("a kind the agent does not take is refused, and Send says so", () => {
  const tray = new Tray();
  tray.add([file("shot.png", "image/png")]);
  tray.target({
    calls: node().calls,
    abilities: {
      steer: false,
      resume: false,
      image: false,
      audio: false,
      fork: false,
    },
    limits: LIMITS,
    agent: "Codex",
  });
  expect(tray.items.get()[0]?.state).toEqual({
    kind: "refused",
    reason: "Codex does not take images",
  });
  expect(tray.ready()).toEqual({
    kind: "blocked",
    reason: "shot.png: Codex does not take images",
  });
  tray.remove(tray.items.get()[0]?.key ?? "");
  expect(tray.ready()).toEqual({ kind: "ready", refs: [] });
});

test("an image whose first bytes are not its type's is not sent", async () => {
  const tray = new Tray();
  tray.add([new File(["not a png"], "shot.png", { type: "image/png" })]);
  const fake = node();
  tray.target({
    calls: fake.calls,
    abilities: {
      steer: false,
      resume: false,
      image: true,
      audio: false,
      fork: false,
    },
    limits: LIMITS,
    agent: "Codex",
  });
  expect(await tray.upload()).toEqual({
    kind: "blocked",
    reason: "shot.png: Not a PNG image",
  });
  expect(fake.most()).toBe(0);
});

test("an empty tray is ready with no files", () => {
  expect(trayReady([])).toEqual({ kind: "ready", refs: [] });
});

test("the new-session page's files go to the session it made", () => {
  const tray = trayOf("new");
  tray.add([file("a.txt")]);
  handOver("new", "s1");
  expect(trayOf("s1")).toBe(tray);
  expect(trayOf("new")).not.toBe(tray);
});

test("a retarget cannot reuse the finished reference or relabel an in-flight upload", async () => {
  const tray = new Tray();
  tray.add([file("retained.txt")]);
  const first = node();
  const second = node();
  const original = fixtureId("NodeId", "original");
  tray.target({
    node: original,
    calls: first.calls,
    abilities: null,
    limits: LIMITS,
    agent: "fake",
  });
  const uploading = tray.upload();
  await first.begun;
  tray.target({
    node: fixtureId("NodeId", "replacement"),
    calls: second.calls,
    abilities: null,
    limits: LIMITS,
    agent: "fake",
  });
  await first.release();
  expect(await uploading).toMatchObject({ kind: "blocked" });
  expect(second.most()).toBe(0);
  expect(tray.items.get()[0]?.state).toMatchObject({
    kind: "ready",
    node: original,
  });
});
