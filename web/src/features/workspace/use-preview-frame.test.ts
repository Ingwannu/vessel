// What an HTML preview panel shows between posts (ADR 0122): the answer's
// frame for its document; while a new document is out, the last ready
// frame, else loading.
import { expect, test } from "vitest";

import { frameOf } from "./use-preview-frame";

const url = `/preview/${"0a".repeat(16)}`;

test("the answer for the document shown is its frame", () => {
  expect(frameOf(null, "a")).toEqual({ status: "loading" });
  expect(frameOf({ document: "a", url }, "a")).toEqual({
    status: "ready",
    url,
  });
  expect(frameOf({ document: "a", url: null }, "a")).toEqual({
    status: "static",
  });
});

test("while a new document is out, the last ready frame stays", () => {
  expect(frameOf({ document: "a", url }, "b")).toEqual({
    status: "ready",
    url,
  });
  expect(frameOf({ document: "a", url: null }, "b")).toEqual({
    status: "loading",
  });
});
