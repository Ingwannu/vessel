// A covered snapshot retires stale cards without inventing resolution records.
// It may supply a dock request whose announcement was pruned, with no fake seq.
import type { Conversation, PermissionRequest, Question } from "../wire/proto";
import type { Asked } from "./conversation";
import { firstPending, type Entry, type Timeline } from "./timeline-model";

export function reconcileSnapshot(
  timeline: Timeline,
  snapshot: Conversation,
): Timeline {
  const through = snapshot.through;
  if (
    through == null ||
    timeline.next < through ||
    (timeline.authoritative?.through ?? -1) >= through
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
  const fromSnapshot = snapshot.permissions.filter(
    (request) => !timeline.permissions.has(request.id),
  );
  const asked: Asked[] = timeline.conversation.questions.map((item) => {
    if (item.outcome !== null || (item.observed ?? item.seq ?? -1) >= through)
      return item;
    const question = questions.get(item.question.id);
    return question === undefined
      ? { ...item, available: false }
      : { ...item, question, available: true, snapshotThrough: through };
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
    authoritative: { through, permissions: fromSnapshot },
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
          request,
          available: true,
          title: request.presentation?.title ?? entry.title,
        };
  }
  const question = questions.get(entry.question.id);
  return question === undefined
    ? { ...entry, available: false }
    : { ...entry, question, available: true };
}
