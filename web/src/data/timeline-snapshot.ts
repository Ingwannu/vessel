// A covered snapshot retires stale cards without inventing resolution records.
// It may supply a dock request whose announcement was pruned, with no fake seq.
import type { Conversation, PermissionRequest, Question } from "../wire/proto";
import type { Asked } from "./conversation";
import { firstPending, type Entry, type Timeline } from "./timeline-model";

export function reconcileSnapshot(
  timeline: Timeline,
  snapshot: Conversation,
  read?: number,
): Timeline {
  const through = snapshot.through;
  const ticket = read ?? through ?? -1;
  if (
    through == null ||
    timeline.next < through ||
    (timeline.authoritative?.through ?? -1) > through ||
    (timeline.authoritative?.read ?? timeline.authoritative?.through ?? -1) >=
      ticket
  )
    return timeline;
  const permissions = new Map(
    snapshot.permissions.map((request) => [request.id, request]),
  );
  const questions = new Map(
    snapshot.questions.map((question) => [question.id, question]),
  );
  const entries = timeline.entries.map((entry) =>
    reconcileEntry(entry, through, permissions, questions),
  );
  const fromSnapshot = snapshot.permissions
    .filter((request) => !timeline.permissions.has(request.id))
    .map((request) => {
      const previous = timeline.authoritative?.permissions.find(
        (item) => item.id === request.id,
      );
      return previous === undefined
        ? request
        : retainAdmission(previous, request);
    });
  const asked: Asked[] = timeline.conversation.questions.map((item) => {
    if (item.outcome !== null || (item.observed ?? item.seq ?? -1) >= through)
      return item;
    const question = questions.get(item.question.id);
    return question === undefined
      ? { ...item, available: false }
      : {
          ...item,
          question: retainAdmission(item.question, question),
          available: true,
          snapshotThrough: through,
        };
  });
  for (const question of snapshot.questions) {
    if (!asked.some((item) => item.question.id === question.id)) {
      const item: Asked = { question, outcome: null, snapshotThrough: through };
      asked.push(item);
    }
  }
  const request = fromSnapshot[0];
  return {
    ...timeline,
    entries,
    authoritative: { through, read: ticket, permissions: fromSnapshot },
    pending:
      request === undefined
        ? firstPending(entries, timeline.permissions)
        : { request, title: request.presentation?.title ?? "Approval" },
    conversation: { ...timeline.conversation, questions: asked },
  };
}
function reconcileEntry(
  entry: Entry,
  through: number,
  permissions: ReadonlyMap<string, PermissionRequest>,
  questions: ReadonlyMap<string, Question>,
): Entry {
  if (entry.kind !== "permission" && entry.kind !== "question") return entry;
  if (entry.outcome !== null || (entry.observed ?? entry.seq) >= through)
    return entry;
  if (entry.kind === "permission") {
    const request = permissions.get(entry.request.id);
    return request === undefined
      ? { ...entry, available: false }
      : {
          ...entry,
          request: retainAdmission(entry.request, request),
          available: true,
          title: request.presentation?.title ?? entry.title,
        };
  }
  const question = questions.get(entry.question.id);
  return question === undefined
    ? { ...entry, available: false }
    : {
        ...entry,
        question: retainAdmission(entry.question, question),
        available: true,
      };
}

// A newer event or an exact no-effect refusal can reopen a response. A read's
// coverage fence cannot: retained delivery may commit independently of the log.
function retainAdmission<
  T extends {
    delivery_state?: "pending" | "answering" | "outcome_unknown" | null;
  },
>(previous: T, incoming: T): T {
  const rank = (
    state: "pending" | "answering" | "outcome_unknown" | null | undefined,
  ) => (state === "outcome_unknown" ? 2 : state === "answering" ? 1 : 0);
  return rank(previous.delivery_state) > rank(incoming.delivery_state)
    ? { ...incoming, delivery_state: previous.delivery_state }
    : incoming;
}
