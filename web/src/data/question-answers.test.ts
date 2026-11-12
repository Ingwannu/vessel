import { expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { Question } from "../wire/proto";
import { answeredLines } from "./question-answers";

const QUESTION: Question = {
  id: fixtureId("QuestionId", "q1"),
  message: "Which way?",
  fields: [
    {
      name: fixtureId("FieldName", "way"),
      title: "Which way",
      required: true,
      kind: "choice",
      options: [
        { value: "left", title: "Left" },
        { value: "right", title: "Right" },
      ],
      multiple: false,
    },
    {
      name: fixtureId("FieldName", "sure"),
      title: "Sure",
      required: false,
      kind: "boolean",
    },
    { name: fixtureId("FieldName", "note"), required: false, kind: "text" },
  ],
};

test("answers by title, a choice by its option's title, in field order", () => {
  expect(
    answeredLines(QUESTION, {
      outcome: "answered",
      values: {
        [fixtureId("FieldName", "sure")]: { type: "boolean", value: true },
        [fixtureId("FieldName", "way")]: { type: "choice", value: "left" },
      },
    }),
  ).toEqual([
    { field: "Which way", value: "Left" },
    { field: "Sure", value: "Yes" },
  ]);
  expect(answeredLines(QUESTION, { outcome: "declined" })).toEqual([]);
});
