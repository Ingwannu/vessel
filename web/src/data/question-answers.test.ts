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

test("Claude's free text joins its question's line", () => {
  const claude: Question = {
    id: fixtureId("QuestionId", "q2"),
    message: "Please answer the following questions.",
    fields: [
      {
        name: fixtureId("FieldName", "question_1"),
        title: "Extras",
        required: false,
        kind: "choice",
        options: [
          { value: "metrics", title: "Metrics" },
          { value: "backups", title: "Backups" },
        ],
        multiple: true,
      },
      {
        name: fixtureId("FieldName", "question_1_custom"),
        title: "Other",
        required: false,
        kind: "text",
      },
      {
        name: fixtureId("FieldName", "question_0"),
        title: "Database",
        required: false,
        kind: "text",
      },
      {
        name: fixtureId("FieldName", "question_0_custom"),
        required: false,
        kind: "text",
      },
    ],
  };
  expect(
    answeredLines(claude, {
      outcome: "answered",
      values: {
        [fixtureId("FieldName", "question_0")]: {
          type: "text",
          value: "SQLite",
        },
        [fixtureId("FieldName", "question_0_custom")]: {
          type: "text",
          value: "",
        },
        [fixtureId("FieldName", "question_1")]: {
          type: "choices",
          value: ["metrics"],
        },
        [fixtureId("FieldName", "question_1_custom")]: {
          type: "text",
          value: "and alerts",
        },
      },
    }),
  ).toEqual([
    { field: "Database", value: "SQLite" },
    { field: "Extras", value: "Metrics, and alerts" },
  ]);
});
