// A typed response receipt is separate from a legacy transport acknowledgement.
// Neither a cancelled decision nor a different request/option is acceptance.
import type {
  AnswerValue,
  PermissionOptionId,
  PermissionRequest,
  Question,
  QuestionAnswer,
  RespondPermissionReply,
  RespondQuestionReply,
  SessionId,
} from "../wire/proto";
import type { NodeKey } from "../wire/bridge";
import type { Fleet } from "./fleet";
import { answerPermission } from "./calls";
import { answerQuestion } from "./conversation-calls";
import { attempt, type Outcome } from "./outcome";

export type ResponseState =
  | "applied"
  | "legacy_sent"
  | "refused"
  | "conflict"
  | "vanished"
  | "uncertain"
  | "answering";
export function permissionNeedsReceipt(request: PermissionRequest): boolean {
  return (
    !request.tool_call ||
    request.tool_scope != null ||
    request.presentation != null ||
    request.plan_proposal != null ||
    request.delivery_state !== undefined ||
    request.options.some((option) => option.action != null)
  );
}
export function questionNeedsReceipt(question: Question): boolean {
  return (
    question.lifetime === "session" ||
    question.tool_scope != null ||
    question.delivery_state !== undefined ||
    question.fields.some(
      (field) =>
        field.presentation != null ||
        (field.kind === "choice" &&
          field.options.some((option) => option.preview != null)),
    )
  );
}
/** A canonical request never falls back after a rejected or uncertain new call. */
export async function respondApproval(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  request: PermissionRequest,
  option: PermissionOptionId,
): Promise<Outcome<RespondPermissionReply | null>> {
  if (!permissionNeedsReceipt(request))
    return answerPermission(fleet, node, session, request.id, option);
  return attempt(() =>
    fleet.bridge.call(node, "session.respond_permission", {
      session,
      permission: request.id,
      response: {
        outcome: { outcome: "selected", option },
        interrupt: "continue",
      },
    }),
  );
}
export async function respondQuestion(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  question: Question,
  answer: QuestionAnswer,
): Promise<Outcome<RespondQuestionReply | null>> {
  if (!questionNeedsReceipt(question))
    return answerQuestion(fleet, node, session, question.id, answer);
  return attempt(() =>
    fleet.bridge.call(node, "session.respond_question", {
      session,
      question: question.id,
      answer,
    }),
  );
}
export function permissionReceipt(
  request: PermissionRequest,
  option: PermissionOptionId,
  reply: RespondPermissionReply | null,
): ResponseState {
  if (reply === null) return "legacy_sent";
  if (reply.permission !== request.id) return "uncertain";
  switch (reply.status) {
    case "settled":
      return reply.outcome.outcome === "selected" &&
        reply.outcome.option === option
        ? "applied"
        : "conflict";
    case "refused":
      return "refused";
    case "conflict":
      return "conflict";
    case "vanished":
      return "vanished";
    case "outcome_unknown":
      return "uncertain";
  }
}
export function questionReceipt(
  question: Question,
  answer: QuestionAnswer,
  reply: RespondQuestionReply | null,
): ResponseState {
  if (reply === null) return "legacy_sent";
  if (reply.question !== question.id) return "uncertain";
  switch (reply.status) {
    case "settled": {
      const outcome = reply.outcome;
      if (answer.answer === "decline")
        return outcome.outcome === "declined" ? "applied" : "conflict";
      return outcome.outcome === "answered" &&
        sameValues(answer.values, outcome.values)
        ? "applied"
        : "conflict";
    }
    case "refused":
      return "refused";
    case "answering":
      return "answering";
    case "outcome_unknown":
      return "uncertain";
    case "conflict":
      return "conflict";
    case "vanished":
      return "vanished";
  }
}
function sameValues(
  expected: Partial<Record<string, AnswerValue>>,
  reported: Partial<Record<string, AnswerValue>>,
): boolean {
  if (Object.keys(expected).length !== Object.keys(reported).length)
    return false;
  return Object.entries(expected).every(([key, value]) => {
    const other = reported[key];
    if (value === undefined || other === undefined || value.type !== other.type)
      return false;
    return value.type === "choices"
      ? other.type === "choices" &&
          value.value.length === other.value.length &&
          value.value.every((choice, index) => choice === other.value[index])
      : value.value === other.value;
  });
}
