// A settled question's answers as the timeline lists them
// (agent-orchestrator's answered card): each field the person answered,
// by its title, and its value in the agent's own words (a choice by its
// option's title).

import type { AnswerValue, Question, QuestionOutcome } from "../wire/proto";

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

/** The answers of `outcome`, in the question's field order; empty for a
 * question skipped or closed. */
export function answeredLines(
  question: Question,
  outcome: QuestionOutcome,
): AnsweredLine[] {
  if (outcome.outcome !== "answered") {
    return [];
  }
  const lines: AnsweredLine[] = [];
  for (const field of question.fields) {
    const value = outcome.values[field.name];
    if (value !== undefined) {
      lines.push({
        field: field.title ?? field.name,
        value: valueText(question, field.name, value),
      });
    }
  }
  return lines;
}
