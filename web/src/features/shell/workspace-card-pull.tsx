// The workspace card's pull request row (TRACKER 426 on the page):
// Paseo's `PrBadge` (`components/sidebar-workspace-list.tsx` 297-356,
// pinned 0.15.4) as the hover card places it, on the card's row rails.
//
// [Decision Log]
// - Purpose: the badge says the pull request's state by its glyph and
//   colour and opens its page, as Paseo's: on hover the words brighten
//   and the glyph becomes the outward arrow, the same size, so the
//   target does not move.
// - Choice: a link when the summary carries a page (a new tab), named
//   "Pull request #N"; the swap is CSS on the link's hover.
// - Differences: the number keeps the page card's "#" (Paseo's badge
//   says the number alone), as the native card does.

import type { SharedPull } from "../../data/workspace/diff-stat";
import {
  badgeOf,
  pullName,
  type PullBadge,
} from "../../data/workspace/meta-row";
import { Icon, type IconName } from "../../ui/kit";

const GLYPHS: Record<PullBadge, IconName> = {
  open: "git-pull-request",
  merged: "git-merge",
  closed: "git-pull-request-closed",
  unknown: "git-pull-request",
};

export function CardPull({ pull }: { pull: SharedPull }) {
  const badge = badgeOf(pull);
  const marks = (
    <span className="workspace-card-mark">
      <span className={`workspace-card-rest tone-${badge}`}>
        <Icon name={GLYPHS[badge]} size={12} />
      </span>
      <span className="workspace-card-out">
        <Icon name="move" size={12} />
      </span>
    </span>
  );
  const number = <span>{`#${String(pull.number)}`}</span>;
  if (pull.url === null) {
    return (
      <div className="workspace-card-row" aria-label={pullName(pull.number)}>
        {marks}
        {number}
      </div>
    );
  }
  return (
    <a
      className="workspace-card-row workspace-card-link"
      href={pull.url}
      target="_blank"
      rel="noreferrer"
      aria-label={pullName(pull.number)}
    >
      {marks}
      {number}
    </a>
  );
}
