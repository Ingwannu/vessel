// The HTML preview's post (ADR 0122): what it sends, what it accepts back,
// and that a tab not connected to a proven server sends nothing.
import { expect, test, vi } from "vitest";

import { Bridge } from "./client";
import { postPreview, type Fetch } from "./preview";
import { fixtureId } from "../testing/ids";

const key = "a".repeat(64);
const address = `/preview/${"0f".repeat(16)}`;

function answering(
  status: number,
  body: unknown,
): {
  send: Fetch;
  seen: { input: string; init: RequestInit }[];
} {
  const seen: { input: string; init: RequestInit }[] = [];
  const send: Fetch = (input, init) => {
    seen.push({ input, init });
    return Promise.resolve(
      new Response(JSON.stringify(body), {
        status,
        headers: { "Content-Type": "application/json" },
      }),
    );
  };
  return { send, seen };
}

test("posts the document with the session key and the panel's slot", async () => {
  const { send, seen } = answering(200, { url: address });
  await expect(postPreview(key, "slot-1", "<p>hi</p>", send)).resolves.toBe(
    address,
  );
  expect(seen).toHaveLength(1);
  const first = seen.at(0);
  expect(first?.input).toBe("/api/preview");
  const init = first?.init;
  expect(init?.method).toBe("POST");
  expect(init?.body).toBe("<p>hi</p>");
  expect(init?.headers).toEqual({
    Authorization: `Bearer ${key}`,
    "Content-Type": "text/html; charset=utf-8",
    "X-Vessel-Preview-Slot": "slot-1",
  });
  expect(init?.referrerPolicy).toBe("no-referrer");
});

test("a refusal or an address elsewhere is not framed", async () => {
  await expect(
    postPreview(key, "s", "x", answering(401, {}).send),
  ).rejects.toThrow("401");
  for (const url of [
    "https://evil.example/preview/00",
    "/preview/../api/status",
    `${address}?x`,
  ]) {
    await expect(
      postPreview(key, "s", "x", answering(200, { url }).send),
    ).rejects.toThrow("no preview address");
  }
});

test("a tab whose socket has not proved its server posts nothing", async () => {
  const send = vi.fn();
  vi.stubGlobal("fetch", send);
  const bridge = new Bridge(
    {
      kind: "session",
      key,
      launch: fixtureId("LaunchId", "b".repeat(64)),
    },
    "ws://127.0.0.1:9/api/ws",
  );
  await expect(bridge.preview("s", "<p>x</p>")).rejects.toThrow(
    "Not connected",
  );
  expect(send).not.toHaveBeenCalled();
  vi.unstubAllGlobals();
});
