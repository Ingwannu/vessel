// A settled question's answers as the timeline lists them
// (agent-orchestrator's answered card): each field the person answered,
// by its title, and its value in the agent's own words (a choice by its
// option's title). Claude's free text (`question_N_custom`) is not a line
// of its own: it joins its question's line, as the dock drew it there as
// that question's "Other" row.

import type { AnswerValue, Question, QuestionOutcome } from "../wire/proto";
import { stepsOf } from "./question-form";

export interface AnsweredLine {
  field: string;
  value: string;
}

function valueText(
  question: Question,
  name: string,
  value: AnswerValue,
): string {
  const field = question.fields.find((candidate) => candidate.name === name);
  const titleOf = (option: string) =>
    field?.kind === "choice"
      ? (field.options.find((choice) => choice.value === option)?.title ??
        option)
      : option;
  switch (value.type) {
    case "text":
      return value.value;
    case "number":
    case "integer":
      return String(value.value);
    case "boolean":
      return value.value ? "Yes" : "No";
    case "choice":
      return titleOf(value.value);
    case "choices":
      return value.value.map(titleOf).join(", ");
  }
}

/** The answers of `outcome`, in the dock's order; empty for a question
 * skipped or closed. */
export function answeredLines(
  question: Question,
  outcome: QuestionOutcome,
): AnsweredLine[] {
  if (outcome.outcome !== "answered") {
    return [];
  }
  const lines: AnsweredLine[] = [];
  for (const { field, other } of stepsOf(question)) {
    const parts = [field, ...(other === null ? [] : [other])].flatMap(
      (part) => {
        const value = outcome.values[part.name];
        if (value === undefined) {
          return [];
        }
        const text = valueText(question, part.name, value);
        return text === "" ? [] : [text];
      },
    );
    if (parts.length > 0) {
      lines.push({ field: field.title ?? field.name, value: parts.join(", ") });
    }
  }
  return lines;
}
