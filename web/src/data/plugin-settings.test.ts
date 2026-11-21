import { describe, expect, it, vi } from "vitest";

import { Bridge } from "../bridge/client";
import { fixtureId } from "../testing/ids";
import type { PluginSettingsRead } from "../wire/proto";
import { Fleet } from "./fleet";
import {
  boolAt,
  draftKey,
  keepDraft,
  rebaseDrafts,
  settingsDrafts,
  textAt,
  withValue,
} from "./plugin-settings";
import {
  docEntry,
  loadDoc,
  resetDoc,
  saveDoc,
  settingsDocs,
} from "./plugin-settings-docs";

const at = {
  node: fixtureId("NodeKey", "laptop"),
  plugin: "prefs",
  settings: "display",
};
const first = fixtureId("SettingsRevision", "a".repeat(64));
const second = fixtureId("SettingsRevision", "b".repeat(64));

function fleet(): Fleet {
  return new Fleet(
    new Bridge(
      { kind: "bootstrap", key: "a".repeat(64) },
      "ws://127.0.0.1:1/api/ws",
    ),
  );
}

describe("a bound settings value", () => {
  it("is read with its type, or marked as another type", () => {
    const values = { on: true, color: "red", size: 3 };
    expect(boolAt(values, "on")).toEqual({ ok: true, value: true });
    expect(boolAt(values, "color")).toEqual({ ok: false });
    expect(textAt(values, "color")).toEqual({ ok: true, value: "red" });
    expect(textAt(values, "missing")).toEqual({ ok: true, value: "" });
    expect(textAt(values, "size")).toEqual({ ok: false });
    expect(boolAt(null, "on")).toEqual({ ok: false });
  });

  it("changes one property and keeps the others", () => {
    expect(withValue({ color: "red", size: 3 }, "color", "blue")).toEqual({
      color: "blue",
      size: 3,
    });
    expect(withValue([], "on", true)).toEqual({ on: true });
  });
});

describe("an input's draft", () => {
  it("moves to the revision read after a conflict, keeping its text", () => {
    const id = draftKey(at, "name");
    const other = draftKey({ ...at, settings: "other" }, "name");
    keepDraft(id, { text: "Ada", revision: first });
    keepDraft(other, { text: "Bea", revision: first });
    rebaseDrafts(at, second);
    expect(settingsDrafts.get().get(id)).toEqual({
      text: "Ada",
      revision: second,
    });
    expect(settingsDrafts.get().get(other)?.revision).toBe(first);
  });
});

describe("a settings document on the page", () => {
  it("reads, saves at a revision, keeps a conflict's words and resets", async () => {
    const page = fleet();
    const call = vi.spyOn(page.bridge, "call");
    call.mockResolvedValueOnce({
      status: "ready",
      revision: first,
      values: { color: "blue" },
    });
    await loadDoc(page, at);
    expect(docEntry(settingsDocs.get(), at).state).toEqual({
      status: "ready",
      revision: first,
      values: { color: "blue" },
    });

    const words =
      "Settings changed on another client. Reload before saving again.";
    call.mockResolvedValueOnce({ status: "conflict", error: words });
    expect(await saveDoc(page, at, "color", first, { color: "red" })).toBe(
      false,
    );
    const refused = docEntry(settingsDocs.get(), at);
    expect(refused.error).toEqual({ key: "color", words, conflict: true });
    expect(refused.state).toMatchObject({ values: { color: "blue" } });

    call.mockResolvedValueOnce({
      status: "invalid",
      error: "size has the wrong type",
    });
    expect(await saveDoc(page, at, "size", first, { size: "big" })).toBe(false);
    expect(docEntry(settingsDocs.get(), at).error).toEqual({
      key: "size",
      words: "size has the wrong type",
      conflict: false,
    });

    const reset = {
      status: "saved",
      revision: second,
      values: { color: "blue" },
    } as const;
    call.mockResolvedValueOnce(reset);
    expect(await resetDoc(page, at)).toBe(true);
    expect(docEntry(settingsDocs.get(), at)).toEqual({
      state: { status: "ready", revision: second, values: { color: "blue" } },
      saving: null,
      error: null,
    });
    expect(
      call.mock.calls.map(([, method, params]) => [method, params]),
    ).toEqual([
      ["plugin.settings.read", { plugin: "prefs", settings: "display" }],
      [
        "plugin.settings.write",
        {
          plugin: "prefs",
          settings: "display",
          revision: first,
          values: { color: "red" },
        },
      ],
      [
        "plugin.settings.write",
        {
          plugin: "prefs",
          settings: "display",
          revision: first,
          values: { size: "big" },
        },
      ],
      [
        "plugin.settings.reset",
        { plugin: "prefs", settings: "display", revision: first },
      ],
    ]);
  });

  it("is loading until its first read", () => {
    const fresh = { ...at, settings: "fresh" };
    expect(docEntry(settingsDocs.get(), fresh).state).toEqual({
      status: "loading",
    });
  });
});

describe("the page's reads and saves of one document (node review R1, R2)", () => {
  it("drops a read that answers after a newer save", async () => {
    const page = fleet();
    const doc = { ...at, settings: "ordered" };
    const call = vi.spyOn(page.bridge, "call");
    let answerRead: (value: PluginSettingsRead) => void = () => undefined;
    call.mockImplementationOnce(
      () =>
        new Promise<PluginSettingsRead>((resolve) => {
          answerRead = resolve;
        }),
    );
    const reading = loadDoc(page, doc);
    call.mockResolvedValueOnce({
      status: "saved",
      revision: second,
      values: { on: true },
    });
    expect(await saveDoc(page, doc, "on", first, { on: true })).toBe(true);
    answerRead({ status: "ready", revision: first, values: { on: false } });
    await reading;
    expect(docEntry(settingsDocs.get(), doc).state).toEqual({
      status: "ready",
      revision: second,
      values: { on: true },
    });
  });

  it("moves the drafts begun from the saved revision to the new one", async () => {
    const page = fleet();
    const doc = { ...at, settings: "drafts" };
    const third = fixtureId("SettingsRevision", "c".repeat(64));
    keepDraft(draftKey(doc, "name"), { text: "Ada", revision: first });
    keepDraft(draftKey(doc, "older"), { text: "Bea", revision: third });
    vi.spyOn(page.bridge, "call").mockResolvedValueOnce({
      status: "saved",
      revision: second,
      values: { on: true, name: "", older: "" },
    });
    expect(await saveDoc(page, doc, "on", first, { on: true })).toBe(true);
    expect(settingsDrafts.get().get(draftKey(doc, "name"))?.revision).toBe(
      second,
    );
    // A draft from another revision is still a conflict.
    expect(settingsDrafts.get().get(draftKey(doc, "older"))?.revision).toBe(
      third,
    );
  });
});
