// The files attached to the next message, over its text
// (agent-orchestrator's staged attachments: chips on the 4% wash, an
// image's 24px thumbnail or a file's icon, its name and "type · size",
// a remove button), with GOAT's upload band and retry: a chip shows its
// upload's progress, and one that failed says why, with Retry where
// trying again can help.

import "./attachment-strip.css";

import { useStore } from "../../data/store";
import { sizeLabel } from "../../data/attach";
import type { Tray } from "../../data/tray";
import type { TrayItem } from "../../data/tray-item";
import { Icon, IconButton, Tooltip } from "../../ui/kit";

/** What a chip says under its name. */
function stateLine(item: TrayItem): { text: string; problem: boolean } {
  const plain = `${item.kind === "file" ? item.mime : item.kind} · ${sizeLabel(item.size)}`;
  switch (item.state.kind) {
    case "held":
    case "attached":
    case "ready":
      return { text: plain, problem: false };
    case "waiting":
      return { text: "Waiting to upload", problem: false };
    case "uploading": {
      const percent = Math.floor(
        (item.state.received / Math.max(1, item.size)) * 100,
      );
      return { text: `Uploading ${String(percent)}%`, problem: false };
    }
    case "refused":
    case "failed":
      return { text: item.state.reason, problem: true };
  }
}

function Chip({ item, tray }: { item: TrayItem; tray: Tray }) {
  const line = stateLine(item);
  const uploading = item.state.kind === "uploading";
  const progress =
    item.state.kind === "uploading"
      ? item.state.received / Math.max(1, item.size)
      : 0;
  return (
    <li
      className={line.problem ? "attachment-chip problem" : "attachment-chip"}
      aria-busy={uploading || undefined}
    >
      {item.preview === null ? (
        <span className="attachment-icon">
          <Icon name="file" size={14} />
        </span>
      ) : (
        <img className="attachment-thumb" src={item.preview} alt="" />
      )}
      <span className="attachment-text">
        <span className="attachment-name">{item.name}</span>
        <span
          className="attachment-meta"
          role={line.problem ? "alert" : undefined}
        >
          {line.text}
        </span>
      </span>
      {item.state.kind === "failed" && item.state.again === "yes" ? (
        <Tooltip label={`Retry upload for ${item.name}`}>
          <IconButton
            label={`Retry upload for ${item.name}`}
            icon="sync"
            size="xs"
            onClick={() => {
              tray.retry(item.key);
            }}
          />
        </Tooltip>
      ) : null}
      <Tooltip label={`Remove ${item.name}`}>
        <IconButton
          label={`Remove ${item.name}`}
          icon="close"
          size="xs"
          onClick={() => {
            tray.remove(item.key);
          }}
        />
      </Tooltip>
      {uploading ? (
        <span
          className="attachment-band"
          style={{ transform: `scaleX(${String(progress)})` }}
          aria-hidden="true"
        />
      ) : null}
    </li>
  );
}

export function AttachmentStrip({ tray }: { tray: Tray }) {
  const items = useStore(tray.items, (value) => value);
  if (items.length === 0) {
    return null;
  }
  return (
    <ul className="attachment-strip" aria-label="Attached files">
      {items.map((item) => (
        <Chip key={item.key} item={item} tray={tray} />
      ))}
    </ul>
  );
}
