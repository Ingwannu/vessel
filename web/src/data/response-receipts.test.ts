import { expect, test } from "vitest";
import { fixtureId } from "../testing/ids";
import type {
  PermissionRequest,
  Question,
  QuestionAnswer,
} from "../wire/proto";
import { permissionReceipt, questionReceipt } from "./response-receipts";
const request: PermissionRequest = {
  id: fixtureId("PermissionId", "approval"),
  tool_call: null,
  options: [],
};
const option = fixtureId("PermissionOptionId", "implement");
test("a cancelled or different option cannot become implementation acceptance", () => {
  expect(
    permissionReceipt(request, option, {
      status: "settled",
      permission: request.id,
      outcome: { outcome: "cancelled" },
      seq: 4,
    }),
  ).toBe("conflict");
  expect(
    permissionReceipt(request, option, {
      status: "settled",
      permission: request.id,
      outcome: {
        outcome: "selected",
        option: fixtureId("PermissionOptionId", "dismiss"),
      },
      seq: 4,
    }),
  ).toBe("conflict");
  expect(
    permissionReceipt(request, option, {
      status: "settled",
      permission: fixtureId("PermissionId", "other"),
      outcome: { outcome: "selected", option },
      seq: 4,
    }),
  ).toBe("uncertain");
  expect(permissionReceipt(request, option, null)).toBe("legacy_sent");
});
test("question settlement compares canonical values independent of map order", () => {
  const question: Question = {
    id: fixtureId("QuestionId", "question"),
    message: "Choose",
    fields: [],
  };
  const answer: QuestionAnswer = {
    answer: "accept",
    values: {
      first: { type: "text", value: "left" },
      second: { type: "choices", value: ["a", "b"] },
    },
  };
  expect(
    questionReceipt(question, answer, {
      status: "settled",
      question: question.id,
      seq: 5,
      outcome: {
        outcome: "answered",
        values: {
          second: { type: "choices", value: ["a", "b"] },
          first: { type: "text", value: "left" },
        },
      },
    }),
  ).toBe("applied");
  expect(
    questionReceipt(question, answer, {
      status: "settled",
      question: question.id,
      seq: 5,
      outcome: { outcome: "declined" },
    }),
  ).toBe("conflict");
});
