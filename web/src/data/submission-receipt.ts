// Validate the actual keyed prompt acceptance; command receipts never start a prompt.
import type { SubmissionDelivery, SubmitPromptReply } from "../wire/proto";

export function promptReceiptFits(
  delivery: SubmissionDelivery,
  reply: Extract<SubmitPromptReply, { status: "accepted" }>,
): boolean {
  const accepted = reply.receipt.accepted;
  if (accepted.kind !== "prompt") return false;
  switch (accepted.disposition.disposition) {
    case "started":
      return true;
    case "queued":
      return delivery === "queue";
    case "steered":
      return delivery === "steer" || delivery === "steer_or_interrupt";
    case "interrupted_and_started":
      return delivery === "interrupt" || delivery === "steer_or_interrupt";
  }
}
