// One workspace's hover card (TRACKER 819). The sidebar name is the
// trigger; this is the card. A row is omitted when its data is absent.
// A copy row is the pressable: the copy glyph shows on hover and a
// check replaces it after a click. Checks draw a hairline, then count
// pills and an outward link when the summary carries a tally, and the
// summary word until then.

import "./workspace-card.css";

import { useEffect, useState } from "react";

import {
  checksName,
  checksPage,
  formatDiffCount,
  type CheckCounts,
  type SharedPull,
} from "../../data/workspace/diff-stat";
import { Icon, type IconName } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { DiffStat } from "../../wire/proto";
import { CardPull } from "./workspace-card-pull";

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
      {pull === null ? null : <CardPull pull={pull} />}
      {stat == null ? null : (
        <div className="workspace-card-row">
          <Icon name="git-compare" size={12} />
          <span className="workspace-card-diff">
            <span className="workspace-card-add">
              {`+${formatDiffCount(stat.additions)}`}
            </span>
            <span className="workspace-card-del">
              {`-${formatDiffCount(stat.deletions)}`}
            </span>
          </span>
        </div>
      )}
      <div className="workspace-card-row">
        <Icon name="machine" size={12} />
        <span>{machine}</span>
      </div>
      {branch === null ? null : (
        <CopyRow
          icon="git-branch"
          value={branch}
          text={branch}
          label="Copy branch"
        />
      )}
      <CopyRow
        icon="folder"
        value={folder.name}
        text={folder.path}
        label="Copy folder"
      />
      <Checks
        checks={pull?.checks ?? null}
        counts={pull?.counts ?? null}
        kind={pull?.kind ?? null}
        url={pull?.url ?? null}
      />
    </div>
  );
}

/** The whole row copies. Hover swaps in the copy glyph; a click swaps
 * in a check for two seconds. Pointer focus draws no box. */
function CopyRow({
  icon,
  value,
  text,
  label,
}: {
  icon: IconName;
  value: string;
  text: string;
  label: string;
}) {
  const [copied, setCopied] = useState(false);
  const [hovered, setHovered] = useState(false);
  useEffect(() => {
    if (!copied) {
      return undefined;
    }
    const timer = window.setTimeout(() => {
      setCopied(false);
    }, 2000);
    return () => {
      window.clearTimeout(timer);
    };
  }, [copied]);
  let mark: IconName = icon;
  if (copied) {
    mark = "check";
  } else if (hovered) {
    mark = "copy";
  }
  return (
    <button
      type="button"
      className="workspace-card-row workspace-card-copy"
      aria-label={label}
      data-copied={copied ? "true" : "false"}
      onClick={() => {
        navigator.clipboard.writeText(text).then(
          () => {
            setCopied(true);
          },
          () => {
            toast("The clipboard refused the text", "error");
          },
        );
      }}
      onPointerEnter={() => {
        setHovered(true);
      }}
      onPointerLeave={() => {
        setHovered(false);
      }}
    >
      <Icon name={mark} size={12} />
      <span>{value}</span>
    </button>
  );
}

function Checks({
  checks,
  counts,
  kind,
  url,
}: {
  checks: string | null;
  counts: CheckCounts | null;
  kind: string | null;
  url: string | null;
}) {
  const pills = counted(counts);
  if (pills !== null) {
    return (
      <div className="workspace-card-checks">
        <div className="workspace-card-rule" />
        <CountPills counts={pills} kind={kind} url={url} />
      </div>
    );
  }
  const shown = checks === null ? null : CHECKS[checks];
  if (shown == null) {
    return null;
  }
  return (
    <div className="workspace-card-checks">
      <div className="workspace-card-rule" />
      <div className={`workspace-card-row ${shown.tone}`}>
        <Icon name={shown.icon} size={12} />
        <span>{shown.word}</span>
      </div>
    </div>
  );
}

function counted(counts: CheckCounts | null): CheckCounts | null {
  if (counts === null) {
    return null;
  }
  if (counts.passed > 0 || counts.failed > 0 || counts.pending > 0) {
    return counts;
  }
  return null;
}

function CountPills({
  counts,
  kind,
  url,
}: {
  counts: CheckCounts;
  kind: string | null;
  url: string | null;
}) {
  const name = checksName(counts);
  const href = url === null ? null : checksPage(kind ?? "unknown", url);
  // Paseo's lead: the forge's mark tinted muted (GitHub's is one colour),
  // the boxed outward arrow while the row is hovered (TRACKER 426).
  const pills = (
    <>
      <span className="workspace-card-mark">
        <span className="workspace-card-rest">
          <Icon
            name={kind === "github" ? "github" : "git-pull-request"}
            size={12}
          />
        </span>
        {href === null ? null : (
          <span className="workspace-card-out">
            <Icon name="move" size={12} />
          </span>
        )}
      </span>
      <span>Checks</span>
      <span className="workspace-card-pills">
        <Pill
          count={counts.passed}
          icon="circle-check"
          tone="workspace-card-ok"
        />
        <Pill count={counts.failed} icon="circle-x" tone="workspace-card-bad" />
        <Pill
          count={counts.pending}
          icon="circle-dashed"
          tone="workspace-card-pending"
        />
      </span>
    </>
  );
  if (href === null) {
    return (
      <div className="workspace-card-row" aria-label={name}>
        {pills}
      </div>
    );
  }
  return (
    <a
      className="workspace-card-row workspace-card-link"
      href={href}
      target="_blank"
      rel="noreferrer"
      aria-label={name}
    >
      {pills}
    </a>
  );
}

function Pill({
  count,
  icon,
  tone,
}: {
  count: number;
  icon: IconName;
  tone: string;
}) {
  if (count <= 0) {
    return null;
  }
  return (
    <span className={`workspace-card-pill ${tone}`}>
      <Icon name={icon} size={12} />
      <span>{String(count)}</span>
    </span>
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
