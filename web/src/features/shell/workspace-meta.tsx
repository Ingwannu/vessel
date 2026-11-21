// The line under a workspace's name (TRACKER 426 on the page): Paseo's
// `WorkspaceMetaRow` (`components/sidebar/workspace-meta-row/index.tsx`,
// pinned 0.15.4). What is on it and in which order is
// `data/workspace/meta-row.ts`; this draws it.
//
// [Decision Log]
// - Purpose: the row's facts as peers on one quiet line, separated by a
//   dot: the machine, the pull request, its checks, the labels.
// - Choice: Paseo's line: 12 on 16, 6 between items, 3 between a glyph
//   and its words, glyphs at 12 (the host badge's size). The pull request
//   is the one item that navigates, so it is a link to its page that
//   brightens and swaps its state glyph for the outward arrow on hover;
//   the checks are the ring and Paseo's word in the outcome's colour,
//   named "Checks passed / failed / running". Labels lead the line pulled
//   left by the chip's inset, so the first letter sits on the name's rail.
// - Differences: the machine is its name after the machine glyph (the
//   page has no host badge colours); the link opens a new tab.

import "./workspace-meta.css";

import { Fragment, type ReactNode } from "react";

import type { SharedPull } from "../../data/workspace/diff-stat";
import {
  badgeWord,
  CHECK_NAMES,
  CHECK_WORDS,
  metaItems,
  pullName,
  type CheckSummary,
  type MetaItem,
  type PullBadge,
} from "../../data/workspace/meta-row";
import { CheckIndicator, Icon, type IconName } from "../../ui/kit";
import type { Label } from "../../wire/proto";
import { LabelChips } from "../workspace-labels/chip";

export function WorkspaceMeta({
  machine,
  pull,
  chips,
}: {
  /** Its machine's name, only in a fleet of several. */
  machine: string | null;
  pull: SharedPull | null;
  chips: readonly Label[];
}) {
  const items = metaItems({ machine, pull, labels: chips.length });
  if (items.length === 0) {
    return null;
  }
  return (
    <div className="side-workspace-meta workspace-meta">
      {items.map((item, index) => (
        <Fragment key={item.kind}>
          {index > 0 ? (
            <span className="workspace-meta-dot" aria-hidden="true">
              ·
            </span>
          ) : null}
          {draw(item, chips, index === 0)}
        </Fragment>
      ))}
    </div>
  );
}

function draw(
  item: MetaItem,
  chips: readonly Label[],
  leading: boolean,
): ReactNode {
  switch (item.kind) {
    case "host":
      return (
        <span className="workspace-meta-item workspace-meta-host">
          <Icon name="machine" size={12} />
          <span className="workspace-meta-words">{item.name}</span>
        </span>
      );
    case "changeRequest":
      return <PullItem pull={item.pull} badge={item.badge} />;
    case "checks":
      return <ChecksItem summary={item.summary} />;
    case "labels":
      return (
        <span
          className={
            leading
              ? "workspace-meta-labels workspace-meta-leading"
              : "workspace-meta-labels"
          }
        >
          <LabelChips labels={chips} />
        </span>
      );
  }
}

const BADGE_GLYPHS: Record<PullBadge, IconName> = {
  open: "git-pull-request",
  merged: "git-merge",
  closed: "git-pull-request-closed",
  unknown: "git-pull-request",
};

function PullItem({ pull, badge }: { pull: SharedPull; badge: PullBadge }) {
  const word = badgeWord(badge);
  const content = (
    <>
      <span className={`workspace-meta-badge tone-${badge}`}>
        <Icon name={BADGE_GLYPHS[badge]} size={12} />
      </span>
      <span className="workspace-meta-out">
        <Icon name="move" size={12} />
      </span>
      <span>
        {String(pull.number)}
        {word === null ? "" : ` ${word}`}
      </span>
    </>
  );
  if (pull.url === null) {
    return (
      <span
        className="workspace-meta-item workspace-meta-pull"
        aria-label={pullName(pull.number)}
      >
        {content}
      </span>
    );
  }
  return (
    <a
      className="workspace-meta-item workspace-meta-pull workspace-meta-link"
      href={pull.url}
      target="_blank"
      rel="noreferrer"
      aria-label={pullName(pull.number)}
    >
      {content}
    </a>
  );
}

function ChecksItem({ summary }: { summary: CheckSummary }) {
  return (
    <span
      className={`workspace-meta-item workspace-meta-checks tone-${summary.outcome}`}
      role="img"
      aria-label={CHECK_NAMES[summary.outcome]}
    >
      <CheckIndicator outcome={summary.outcome} quarters={summary.quarters} />
      <span aria-hidden="true">{CHECK_WORDS[summary.outcome]}</span>
    </span>
  );
}
