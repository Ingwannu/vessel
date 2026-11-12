import { expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { AttachmentRef } from "../wire/proto";
import { base64Of, send, type UploadCalls } from "./upload";

const REF: AttachmentRef = {
  id: fixtureId("AttachmentId", "a1"),
  name: "notes.txt",
  kind: "file",
  mime: "text/plain",
  size: 10,
};

/** A node that takes pieces of `chunk` bytes and keeps what it got. */
function node(chunk: number, failAt: number | null = null) {
  const got: { offset: number; data: string }[] = [];
  let received = 0;
  const calls: UploadCalls = {
    begin: () =>
      Promise.resolve({
        ok: true,
        value: { upload: fixtureId("UploadId", "u1"), chunk },
      }),
    write: (_upload, offset, data) => {
      if (failAt !== null && offset >= failAt) {
        return Promise.resolve({
          ok: false,
          problem: { from: "op", reason: "failed", message: "gone" },
        });
      }
      got.push({ offset, data });
      received = offset + atob(data).length;
      return Promise.resolve({ ok: true, value: { received } });
    },
    finish: () => Promise.resolve({ ok: true, value: REF }),
  };
  return { calls, got };
}

const TEN = new Blob(["0123456789"]);

test("pieces of the node's size, from what it has, then its record", async () => {
  const { calls, got } = node(4);
  const seen: number[] = [];
  const end = await send(
    calls,
    TEN,
    "notes.txt",
    "text/plain",
    null,
    (received) => seen.push(received),
    () => false,
  );
  expect(end).toEqual({ kind: "done", ref: REF });
  expect(got.map((piece) => [piece.offset, atob(piece.data)])).toEqual([
    [0, "0123"],
    [4, "4567"],
    [8, "89"],
  ]);
  expect(seen).toEqual([4, 8, 10]);
});

test("a failed piece stops the file with the node's reason", async () => {
  const { calls } = node(4, 4);
  const end = await send(
    calls,
    TEN,
    "notes.txt",
    "text/plain",
    null,
    () => undefined,
    () => false,
  );
  expect(end).toMatchObject({
    kind: "failed",
    problem: { message: "gone" },
    again: "yes",
    resume: { received: 4 },
  });
});

test("stopping ends it between pieces", async () => {
  const { calls, got } = node(4);
  let pieces = 0;
  const end = await send(
    calls,
    TEN,
    "notes.txt",
    "text/plain",
    null,
    () => {
      pieces += 1;
    },
    () => pieces >= 1,
  );
  expect(end).toMatchObject({ kind: "stopped", resume: { received: 4 } });
  expect(got).toHaveLength(1);
});

test("a retry goes on from what the node has", async () => {
  const { calls, got } = node(4);
  const end = await send(
    calls,
    TEN,
    "notes.txt",
    "text/plain",
    { upload: fixtureId("UploadId", "u1"), chunk: 4, received: 8 },
    () => undefined,
    () => false,
  );
  expect(end).toEqual({ kind: "done", ref: REF });
  expect(got.map((piece) => piece.offset)).toEqual([8]);
});

test("the node's refusal of the file is not tried again", async () => {
  const refused: UploadCalls = {
    ...node(4).calls,
    begin: () =>
      Promise.resolve({
        ok: false,
        problem: {
          from: "call",
          failure: {
            kind: "remote",
            error: {
              code: "invalid_request",
              message: "over the session's room",
            },
          },
          message: "over the session's room",
        },
      }),
  };
  const end = await send(
    refused,
    TEN,
    "notes.txt",
    "text/plain",
    null,
    () => undefined,
    () => false,
  );
  expect(end).toMatchObject({ kind: "failed", again: "never", resume: null });
});

test("base64 of a piece larger than one argument list", () => {
  const bytes = new Uint8Array(100_000).map((_, index) => index % 251);
  expect(atob(base64Of(bytes)).length).toBe(100_000);
  expect(base64Of(new Uint8Array([255, 0, 1]))).toBe("/wAB");
});
