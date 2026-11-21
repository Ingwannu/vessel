// The files attached to the next message, over its text
// (agent-orchestrator's staged attachments: chips on the 4% wash, an
// image's 24px thumbnail or a file's icon, its name and "type · size",
// a remove button), with GOAT's upload band and retry: a chip shows its
// upload's progress, and one that failed says why, with Retry where
// trying again can help. After the files, a session's context pills
// (TRACKER 833, data/context-tray.ts): Paseo's file-text mark, the
// pill's title, its kind's word or a file's path, and Remove; nothing
// opens on a press (Paseo's pills for these kinds open nothing either).

import "./attachment-strip.css";

import { Store, useStore } from "../../data/store";
import { sizeLabel } from "../../data/attach";
import {
  type ContextPill,
  type ContextTray,
  pillKey,
} from "../../data/context-tray";
import {
  contextWords,
  pillLine,
  removeLabel,
  type ContextWords,
} from "../../data/context-words";
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

function Pill({
  pill,
  context,
  words,
}: {
  pill: ContextPill;
  context: ContextTray;
  words: ContextWords;
}) {
  const remove = removeLabel(pill, words);
  return (
    <li className="attachment-chip context-pill">
      <span className="attachment-icon">
        <Icon name="file-text" size={14} />
      </span>
      <span className="attachment-text">
        <span className="attachment-name">{pill.title}</span>
        <span className="attachment-meta">{pillLine(pill, words)}</span>
      </span>
      <Tooltip label={remove}>
        <IconButton
          label={remove}
          icon="close"
          size="xs"
          onClick={() => {
            context.remove(pillKey(pill));
          }}
        />
      </Tooltip>
    </li>
  );
}

const NO_CONTEXT = new Store<readonly ContextPill[]>([]);

export function AttachmentStrip({
  tray,
  context = null,
}: {
  tray: Tray;
  /** The session's context pills; the new-session page has none. */
  context?: ContextTray | null;
}) {
  const items = useStore(tray.items, (value) => value);
  const pills = useStore(context?.pills ?? NO_CONTEXT, (value) => value);
  if (items.length === 0 && pills.length === 0) {
    return null;
  }
  const words = contextWords(navigator.language);
  return (
    <ul className="attachment-strip" aria-label={words.strip}>
      {items.map((item) => (
        <Chip key={item.key} item={item} tray={tray} />
      ))}
      {context === null
        ? null
        : pills.map((pill) => (
            <Pill
              key={pillKey(pill)}
              pill={pill}
              context={context}
              words={words}
            />
          ))}
    </ul>
  );
}
