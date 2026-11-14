import { describe, expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { Question, QuestionField } from "../wire/proto";
import {
  answerOf,
  readField,
  refusedField,
  startingDrafts,
  stepProblem,
  stepsOf,
} from "./question-form";

const name = (text: string) => fixtureId("FieldName", text);

function choice(field: string, multiple: boolean): QuestionField {
  return {
    name: name(field),
    title: field,
    required: false,
    kind: "choice",
    options: [
      { value: "a", title: "A", description: "the first" },
      { value: "b", title: "B" },
    ],
    multiple,
  };
}

function text(field: string, required = false): QuestionField {
  return { name: name(field), title: "Other", required, kind: "text" };
}

/** Claude's `AskUserQuestion` form as the node passes it: by key. */
const CLAUDE: Question = {
  id: fixtureId("QuestionId", "q1"),
  message: "Please answer the following questions.",
  fields: [
    choice("question_0", false),
    text("question_0_custom"),
    choice("question_10", true),
    text("question_10_custom"),
    choice("question_2", false),
    text("question_2_custom"),
  ],
};

describe("steps", () => {
  test("Claude's pairs are one step each, in their numbers' order", () => {
    const steps = stepsOf(CLAUDE);
    expect(steps.map((step) => step.field.name)).toEqual([
      "question_0",
      "question_2",
      "question_10",
    ]);
    expect(steps.map((step) => step.other?.name)).toEqual([
      "question_0_custom",
      "question_2_custom",
      "question_10_custom",
    ]);
  });

  test("any other form keeps the node's order, a step per field", () => {
    const form: Question = {
      id: fixtureId("QuestionId", "q2"),
      message: "Which way?",
      fields: [text("name"), choice("way", false), text("way_custom")],
    };
    expect(stepsOf(form).map((step) => step.field.name)).toEqual([
      "name",
      "way",
      "way_custom",
    ]);
  });
});

describe("values", () => {
  test("defaults start filled in", () => {
    const field: QuestionField = {
      name: name("n"),
      required: true,
      kind: "integer",
      default: 3,
    };
    const drafts = startingDrafts({ ...CLAUDE, fields: [field] });
    expect(readField(field, drafts[name("n")])).toEqual({
      kind: "value",
      value: { type: "integer", value: 3 },
    });
  });

  test("numbers take a sign and a point, integers no point", () => {
    const number: QuestionField = {
      name: name("x"),
      required: true,
      kind: "number",
      minimum: -5,
    };
    expect(readField(number, { kind: "text", text: "-2.5" })).toEqual({
      kind: "value",
      value: { type: "number", value: -2.5 },
    });
    expect(readField(number, { kind: "text", text: "-6" })).toEqual({
      kind: "wrong",
      why: "At least -5",
    });
    expect(
      readField({ ...number, kind: "integer" }, { kind: "text", text: "1.5" }),
    ).toEqual({ kind: "wrong", why: "A whole number" });
  });

  test("a required step with nothing, and Claude's Other answering it", () => {
    const step = stepsOf({
      ...CLAUDE,
      fields: [
        { ...choice("question_0", false), required: true },
        text("question_0_custom"),
      ],
    })[0];
    expect(step).toBeDefined();
    if (step !== undefined) {
      expect(stepProblem(step, {})).toBe("Choose an answer.");
      expect(
        stepProblem(step, {
          [name("question_0_custom")]: { kind: "text", text: "neither" },
        }),
      ).toBeNull();
    }
  });

  test("the answer holds every field with a value; picks and notes both", () => {
    const answer = answerOf(CLAUDE, {
      [name("question_0")]: { kind: "picked", values: ["a"] },
      [name("question_0_custom")]: { kind: "text", text: "and quickly" },
      [name("question_10")]: { kind: "picked", values: ["a", "b"] },
    });
    expect(answer).toEqual({
      answer: "accept",
      values: {
        question_0: { type: "choice", value: "a" },
        question_0_custom: { type: "text", value: "and quickly" },
        question_10: { type: "choices", value: ["a", "b"] },
      },
    });
  });
});

test("a refusal names its field", () => {
  expect(
    refusedField(CLAUDE, "field `question_2`: a value of another kind"),
  ).toBe("question_2");
  expect(refusedField(CLAUDE, "something else")).toBeNull();
});
