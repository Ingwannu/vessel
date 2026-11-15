// A question in the timeline: one line in the review colour while it
// waits (its form is docked over the composer, question-dock.tsx), then
// agent-orchestrator's answered card: the question, each answer, and how
// it was settled.

import { answeredLines } from "../../data/question-answers";
import type { Entry } from "../../data/timeline-model";
import { questionLine } from "../../data/timeline-text";

export function QuestionEntry({
  entry,
}: {
  entry: Extract<Entry, { kind: "question" }>;
}) {
  if (entry.available === false && entry.outcome === null)
    return (
      <div className="t-item meta-item" data-entry={entry.key}>
        <p className="t-meta">
          Question no longer pending · {entry.question.message}
        </p>
      </div>
    );
  if (entry.outcome === null) {
    return (
      <div className="t-item meta-item perm pending" data-entry={entry.key}>
        <span className="glyph" aria-hidden="true" />
        <p className="t-meta">
          Waiting on an answer · {entry.question.message}
        </p>
      </div>
    );
  }
  const lines = answeredLines(entry.question, entry.outcome);
  return (
    <div className="t-item meta-item perm" data-entry={entry.key}>
      <span className="glyph" aria-hidden="true" />
      <div className="question-card">
        <p className="question-card-ask">{entry.question.message}</p>
        {lines.length === 0 ? null : (
          <dl className="question-card-answers">
            {lines.map((line) => (
              <div key={line.field}>
                <dt>{line.field}</dt>
                <dd>{line.value}</dd>
              </div>
            ))}
          </dl>
        )}
        <p className="question-card-outcome">{questionLine(entry.outcome)}</p>
      </div>
    </div>
  );
}
