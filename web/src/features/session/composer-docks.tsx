// What stands over a running session's composer, top to bottom: the
// saved login's limit notice (features/usage/limit-notice.tsx), the
// prompts waiting in the node's queue, then a permission the agent waits
// on, or else the oldest question it asks (one at a time; the permission
// first, since the turn cannot go on without it).

import { waitingQuestions } from "../../data/conversation";
import { heldBy, type QueueRow } from "../../data/queue-view";
import type { Timeline } from "../../data/timeline-model";
import type { NodeKey } from "../../wire/bridge";
import type { QueueEntryId, SessionId } from "../../wire/proto";
import { LimitNotice } from "../usage/limit-notice";
import { pendingTool } from "./composer-props";
import { PermissionDock } from "./permission-dock";
import { QuestionDock } from "./question-dock";
import { type QueueActions, QueueDock } from "./queue-dock";

export function ComposerDocks({
  node,
  session,
  timeline,
  rows,
  editing,
  actions,
}: {
  node: NodeKey;
  session: SessionId;
  timeline: Timeline;
  rows: readonly QueueRow[];
  editing: QueueEntryId | null;
  actions: QueueActions;
}) {
  const pending = timeline.pending;
  const questions = waitingQuestions(timeline.conversation);
  const question = questions[0];
  const tool = pendingTool(timeline);
  return (
    <>
      <LimitNotice node={node} session={session} binding={timeline.binding} />
      <QueueDock
        rows={rows}
        held={heldBy(timeline.conversation.queue)}
        editing={editing}
        actions={actions}
      />
      {pending !== null ? (
        <PermissionDock
          node={node}
          session={session}
          entry={pending}
          location={
            tool?.kind === "tool"
              ? (tool.tool.locations[0]?.path ?? null)
              : null
          }
        />
      ) : question === undefined ? null : (
        <QuestionDock
          key={question.question.id}
          node={node}
          session={session}
          question={question.question}
          waiting={questions.length - 1}
        />
      )}
    </>
  );
}
