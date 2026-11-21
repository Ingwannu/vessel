// The Pull request pane's checks (TRACKER 430 on the page): Paseo's
// `pull-request-panel/checks-section.tsx` and `checks-ring.tsx` (pinned
// 0.15.4). The rules are `data/workspace/pull/checks.ts`.
//
// [Decision Log]
// - Purpose: the worst outcome first, as a headline beside the ring, then
//   each presentation's checks in Paseo's order with their timing, and a
//   failed check's logs a press away from the composer.
// - Choice: the ring is one SVG circle per segment, dashed to its length
//   from twelve o'clock; groups fold under their header (open at first);
//   a row links to the check's page; "Add logs to chat" asks the node for
//   the report (`pr.check`) and puts Paseo's check pill in the composer.
// - Differences: no live timer (a running check's time is as of the last
//   read); the report is fetched per press, not cached.

import { type ReactNode, useEffect, useRef, useState } from "react";

import {
  checkAttachment,
  canAddCheck,
} from "../../../data/workspace/pull/attach-check";
import type { About, Attachment } from "../../../data/workspace/pull/attach";
import {
  checksSummary,
  circumference,
  RING_RADIUS,
  RING_SIZE,
  RING_STROKE,
  segments,
  summaryParts,
  timing,
  type Presentation,
} from "../../../data/workspace/pull/checks";
import {
  detailWords,
  groupWords,
  timingWords,
} from "../../../data/workspace/pull/checks-words";
import { outcomeWords } from "../../../data/workspace/pull/facts-words";
import type { PullWords } from "../../../data/workspace/pull/words";
import { Button, Icon, type IconName } from "../../../ui/kit";
import { toast } from "../../../ui/toasts";
import type { CheckRun, Checks, RunId } from "../../../wire/proto";
import type { Outcome } from "../../../data/outcome";
import type { CheckDetails } from "../../../wire/proto";

const MARKS: Record<Presentation, IconName> = {
  failure: "circle-x",
  pending: "circle-dashed",
  success: "circle-check",
  ignored: "circle-slash",
};

export function PullChecks({
  checks,
  words,
  now,
  about,
  report,
  add,
}: {
  checks: Checks;
  words: PullWords;
  now: number;
  /** The pull request the logs are about; null without a page. */
  about: About | null;
  report: (run: RunId) => Promise<Outcome<CheckDetails>>;
  add: (attachments: readonly Attachment[]) => void;
}) {
  const runs = checks.runs ?? checks.failing;
  const summary = checksSummary(runs);
  const parts = summaryParts(summary);
  return (
    <section className="pull-section" aria-label={words.checks}>
      <div className="pull-checks-head">
        <Ring parts={parts} total={summary.total} />
        <div className="pull-checks-words">
          <span className={`pull-checks-title tone-${summary.outcome}`}>
            {outcomeWords(summary.outcome, words)}
          </span>
          {summary.total === 0 ? null : (
            <span className="pull-muted">{detailWords(summary, words)}</span>
          )}
        </div>
      </div>
      {summary.groups.map((group) => (
        <Group
          key={group.presentation}
          presentation={group.presentation}
          label={groupWords(group.presentation, group.checks.length, words)}
        >
          {group.checks.map((run, index) => (
            <Row
              key={`${run.name}:${String(run.run ?? index)}`}
              run={run}
              words={words}
              now={now}
              about={about}
              report={report}
              add={add}
            />
          ))}
        </Group>
      ))}
    </section>
  );
}

function Ring({
  parts,
  total,
}: {
  parts: ReturnType<typeof summaryParts>;
  total: number;
}) {
  const whole = circumference();
  const center = RING_SIZE / 2;
  return (
    <svg
      className="pull-ring"
      width={RING_SIZE}
      height={RING_SIZE}
      viewBox={`0 0 ${String(RING_SIZE)} ${String(RING_SIZE)}`}
      aria-hidden="true"
    >
      <circle
        className="pull-ring-track"
        cx={center}
        cy={center}
        r={RING_RADIUS}
        fill="none"
        strokeWidth={RING_STROKE}
      />
      {segments(parts, total).map((segment) => (
        <circle
          key={segment.presentation}
          className={`pull-ring-part tone-${segment.presentation}`}
          cx={center}
          cy={center}
          r={RING_RADIUS}
          fill="none"
          strokeWidth={RING_STROKE}
          strokeDasharray={`${String(segment.length)} ${String(whole)}`}
          strokeDashoffset={String(-segment.start)}
          transform={`rotate(-90 ${String(center)} ${String(center)})`}
        />
      ))}
    </svg>
  );
}

function Group({
  presentation,
  label,
  children,
}: {
  presentation: Presentation;
  label: string;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(true);
  return (
    <div className="pull-group">
      <button
        type="button"
        className="pull-group-head"
        aria-expanded={open}
        onClick={() => {
          setOpen(!open);
        }}
      >
        <span className={`pull-mark tone-${presentation}`}>
          <Icon name={MARKS[presentation]} size={14} />
        </span>
        <span>{label}</span>
        <Icon name={open ? "chevron-down" : "chevron"} size={12} />
      </button>
      {open ? <div className="pull-group-rows">{children}</div> : null}
    </div>
  );
}

function Row({
  run,
  words,
  now,
  about,
  report,
  add,
}: {
  run: CheckRun;
  words: PullWords;
  now: number;
  about: About | null;
  report: (run: RunId) => Promise<Outcome<CheckDetails>>;
  add: (attachments: readonly Attachment[]) => void;
}) {
  const [adding, setAdding] = useState(false);
  // A report that comes back after the pane closed adds nothing.
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const took = timing(run, now);
  const name = (
    <span className="pull-row-name">
      <span>{run.name}</span>
      {run.workflow == null ? null : (
        <span className="pull-muted">{run.workflow}</span>
      )}
    </span>
  );
  const addLogs = async () => {
    if (about === null) return;
    setAdding(true);
    const answer = run.run == null ? null : await report(run.run);
    if (!mounted.current) return;
    setAdding(false);
    if (answer !== null && !answer.ok) {
      toast(answer.problem.message, "error");
      return;
    }
    const found = answer?.value.report ?? null;
    add([checkAttachment(about, run, found)]);
  };
  return (
    <div className="pull-row">
      {run.url == null ? (
        name
      ) : (
        <a
          href={run.url}
          target="_blank"
          rel="noreferrer"
          className="pull-link"
        >
          {name}
        </a>
      )}
      {took === null ? null : (
        <span className="pull-muted pull-row-time">
          {timingWords(took, words)}
        </span>
      )}
      {about !== null && canAddCheck(run) ? (
        <Button
          variant="ghost"
          size="xs"
          icon="message"
          busy={adding}
          onClick={() => {
            void addLogs();
          }}
        >
          {adding ? words.addingToChat : words.addToChat}
        </Button>
      ) : null}
    </div>
  );
}
