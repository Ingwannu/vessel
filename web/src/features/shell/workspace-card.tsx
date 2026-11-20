// One workspace's hover card (TRACKER 819). The sidebar name is the
// trigger; this is the card. A row is omitted when its data is absent.

import "./workspace-card.css";

import {
  formatDiffCount,
  type SharedPull,
} from "../../data/workspace/diff-stat";
import { Icon, type IconName } from "../../ui/kit";
import { CopyButton } from "../../ui/parts";
import type { DiffStat } from "../../wire/proto";

export function WorkspaceCard({
  title,
  machine,
  folder,
  branch,
  pull,
  stat,
}: {
  title: string;
  machine: string;
  folder: { path: string; name: string };
  branch: string | null;
  pull: SharedPull | null;
  stat: DiffStat | null | undefined;
}) {
  return (
    <div className="workspace-card">
      <p className="workspace-card-title">{title}</p>
      {pull === null ? null : (
        <div className="workspace-card-row">
          <Icon name="git-fork" size={12} />
          <span>{`#${String(pull.number)}`}</span>
        </div>
      )}
      {stat == null ? null : (
        <div className="workspace-card-row">
          <Icon name="git-compare" size={12} />
          <span className="workspace-card-diff">
            <span className="workspace-card-add">
              {`+${formatDiffCount(stat.additions)}`}
            </span>
            <span className="workspace-card-del">
              {`\u2212${formatDiffCount(stat.deletions)}`}
            </span>
          </span>
        </div>
      )}
      <div className="workspace-card-row">
        <Icon name="machine" size={12} />
        <span>{machine}</span>
      </div>
      {branch === null ? null : (
        <div className="workspace-card-row">
          <Icon name="git-branch" size={12} />
          <span>{branch}</span>
          <CopyButton text={branch} label="Copy branch" />
        </div>
      )}
      <div className="workspace-card-row">
        <Icon name="folder" size={12} />
        <span>{folder.name}</span>
        <CopyButton text={folder.path} label="Copy folder" />
      </div>
      <Checks checks={pull?.checks ?? null} />
    </div>
  );
}

function Checks({ checks }: { checks: string | null }) {
  const shown = checks === null ? null : CHECKS[checks];
  if (shown == null) {
    return null;
  }
  return (
    <div className={`workspace-card-row ${shown.tone}`}>
      <Icon name={shown.icon} size={12} />
      <span>{shown.word}</span>
    </div>
  );
}

const CHECKS: Record<
  string,
  { icon: IconName; word: string; tone: string } | null
> = {
  none: null,
  pending: { icon: "circle-dashed", word: "Checks pending", tone: "" },
  passing: {
    icon: "circle-check",
    word: "Checks passing",
    tone: "workspace-card-ok",
  },
  failing: {
    icon: "circle-x",
    word: "Checks failing",
    tone: "workspace-card-bad",
  },
  cancelled: {
    icon: "circle-x",
    word: "Checks cancelled",
    tone: "workspace-card-bad",
  },
  unknown: { icon: "circle-dashed", word: "Checks unknown", tone: "" },
};
