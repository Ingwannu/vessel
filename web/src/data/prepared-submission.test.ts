import { expect, test } from "vitest";
import { fixtureId } from "../testing/ids";
import type { SubmitPrompt } from "../wire/proto";
import {
  MAX_PREPARED_BYTES,
  MAX_PREPARED_SUBMISSIONS,
  PreparedSubmissions,
  type PreparedInput,
  type PreparedPort,
} from "./prepared-submission";

const node = fixtureId("NodeId", "recipient");
function input(session = "session"): PreparedInput {
  return {
    node,
    session: fixtureId("SessionId", session),
    message: fixtureId("ClientMessageId", `message-${session}`),
    text: "original",
    delivery: "steer_or_interrupt",
    files: [],
  };
}
function file(): PreparedInput["files"][number] {
  return {
    key: "file",
    name: "file.txt",
    mime: "text/plain",
    kind: "file",
    file: new Blob(["hello"]),
    size: 5,
    preview: null,
    state: { kind: "attached" },
  };
}
function reserve(owner: PreparedSubmissions, value = input()) {
  const result = owner.reserve(value);
  if (result.status !== "reserved") throw new Error("Fixture did not reserve");
  return result.handle;
}
function fixture(accepted = false) {
  const count = { check: 0, begin: 0, write: 0, finish: 0, submit: 0 };
  let last: SubmitPrompt | null = null;
  const port: PreparedPort = {
    check: async () => {
      count.check += 1;
      return { ok: true, value: { status: "ready", node } };
    },
    uploads: {
      begin: async () => {
        count.begin += 1;
        return {
          ok: true,
          value: { upload: fixtureId("UploadId", "upload"), chunk: 10 },
        };
      },
      write: async (_upload, offset, data) => {
        count.write += 1;
        return { ok: true, value: { received: offset + atob(data).length } };
      },
      finish: async () => {
        count.finish += 1;
        return {
          ok: true,
          value: {
            id: fixtureId("AttachmentId", "finished"),
            name: "file.txt",
            mime: "text/plain",
            kind: "file",
            size: 5,
          },
        };
      },
    },
    submit: async (request) => {
      count.submit += 1;
      last = request;
      return {
        ok: true,
        value: accepted
          ? {
              status: "accepted",
              receipt: {
                message: request.message,
                accepted: {
                  kind: "prompt",
                  disposition: { disposition: "started" },
                },
              },
            }
          : { status: "outcome_unknown", message: request.message },
      };
    },
  };
  return { count, port, last: () => last };
}

test("unknown retains the captured payload and blob lease without another upload or send", async () => {
  const owner = new PreparedSubmissions();
  const draft = input();
  const files = [file()];
  draft.files = files;
  const handle = reserve(owner, draft);
  draft.text = "changed after capture";
  files.length = 0;
  const transport = fixture();
  expect(await owner.run(handle, transport.port)).toEqual({
    status: "uncertain",
    message: handle.message,
  });
  expect(transport.last()?.content).toEqual([
    { type: "text", text: "original" },
    {
      type: "attachment",
      attachment: {
        id: "finished",
        name: "file.txt",
        mime: "text/plain",
        kind: "file",
        size: 5,
      },
    },
  ]);
  expect(owner.held(node, draft.session)).toBe(handle);
  expect(owner.abandon(handle)).toBe(false);
  expect(owner.reserve(draft)).toEqual({
    status: "held",
    message: handle.message,
  });
  await owner.run(handle, transport.port);
  expect(transport.count).toEqual({
    check: 1,
    begin: 1,
    write: 1,
    finish: 1,
    submit: 1,
  });
});

test("unsupported preflight does no upload and frees a never-dispatched owner", async () => {
  const owner = new PreparedSubmissions();
  const draft = { ...input(), files: [file()] };
  const handle = reserve(owner, draft);
  const transport = fixture();
  transport.port.check = async () => ({
    ok: true,
    value: {
      status: "unsupported",
      failure: { kind: "failed", message: "unassigned" },
    },
  });
  expect(await owner.run(handle, transport.port)).toEqual({
    status: "unsupported",
  });
  expect(transport.count.begin).toBe(0);
  expect(transport.count.submit).toBe(0);
  expect(owner.held(node, draft.session)).toBeUndefined();
});

test("an explicit preparation retry resumes the upload before exactly one model submission", async () => {
  const owner = new PreparedSubmissions();
  const handle = reserve(owner, { ...input(), files: [file()] });
  const transport = fixture(true);
  const write = transport.port.uploads.write;
  let first = true;
  transport.port.uploads.write = (...args) => {
    if (!first) return write(...args);
    first = false;
    return Promise.resolve({
      ok: false,
      problem: {
        from: "call",
        failure: { kind: "disconnected" },
        message: "lost chunk ack",
      },
    });
  };
  expect(await owner.run(handle, transport.port)).toMatchObject({
    status: "preparation_failed",
  });
  expect(transport.count.submit).toBe(0);
  expect(await owner.run(handle, transport.port)).toMatchObject({
    status: "accepted",
  });
  expect(transport.count.begin).toBe(1);
  expect(transport.count.submit).toBe(1);
});

test("count and byte saturation refuse before any port work without evicting uncertainty", async () => {
  const owner = new PreparedSubmissions();
  const transport = fixture();
  for (let n = 0; n < MAX_PREPARED_SUBMISSIONS; n += 1)
    await owner.run(
      reserve(owner, input(`session-${String(n)}`)),
      transport.port,
    );
  expect(owner.reserve(input("overflow"))).toMatchObject({ status: "refused" });
  expect(owner.reserve(input("session-0"))).toMatchObject({ status: "held" });
  expect(transport.count.submit).toBe(MAX_PREPARED_SUBMISSIONS);
  class TooLarge extends Blob {
    override get size() {
      return MAX_PREPARED_BYTES;
    }
  }
  const large = { ...file(), file: new TooLarge() };
  expect(
    new PreparedSubmissions().reserve({ ...input(), files: [large] }),
  ).toMatchObject({ status: "refused" });
});

test("contradictory or mismatched receipts never release the retained prompt", async () => {
  for (const mode of ["command", "message", "delivery"] as const) {
    const owner = new PreparedSubmissions();
    const handle = reserve(owner);
    const transport = fixture(true);
    transport.port.submit = async (request) => ({
      ok: true,
      value: {
        status: "accepted",
        receipt: {
          message:
            mode === "message"
              ? fixtureId("ClientMessageId", "other")
              : request.message,
          accepted:
            mode === "command"
              ? { kind: "command", echo: 5, status: "compact_started" }
              : {
                  kind: "prompt",
                  disposition:
                    mode === "delivery"
                      ? {
                          disposition: "queued",
                          entry: fixtureId("QueueEntryId", "entry"),
                        }
                      : { disposition: "started" },
                },
        },
      },
    });
    expect(await owner.run(handle, transport.port)).toMatchObject({
      status: "uncertain",
    });
    expect(owner.abandon(handle)).toBe(false);
    expect(owner.held(node, input().session)).toBe(handle);
  }
});
