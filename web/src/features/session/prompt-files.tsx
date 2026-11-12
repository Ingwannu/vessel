// The files a prompt carried, over its text in the timeline
// (agent-orchestrator's attachment row above the message): images as
// 64px-tall tiles that open full size, other files as small cards with
// their name, type and size. An image is fetched only when its row comes
// near the view (data/images.ts); one sent from this tab is shown from
// its local file.

import "./prompt-files.css";

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { sizeLabel } from "../../data/attach";
import { keptImage, loadImage } from "../../data/images";
import { Dialog, Icon } from "../../ui/kit";
import type { AttachmentRef } from "../../wire/proto";
import { whenNear } from "./near-view";
import { useTimelineSource } from "./timeline-source";

export function PromptFiles({ files }: { files: readonly AttachmentRef[] }) {
  if (files.length === 0) {
    return null;
  }
  return (
    <ul className="prompt-files" aria-label="Attached files">
      {files.map((file) => (
        <li key={file.id}>
          {file.kind === "image" ? (
            <ImageTile file={file} />
          ) : (
            <FileTile file={file} />
          )}
        </li>
      ))}
    </ul>
  );
}

function FileTile({ file }: { file: AttachmentRef }) {
  return (
    <span className="file-tile">
      <span className="file-tile-icon">
        <Icon name="file" size={14} />
      </span>
      <span className="file-tile-text">
        <span className="file-tile-name">{file.name}</span>
        <span className="file-tile-meta">
          {file.kind === "audio" ? "Audio" : file.mime} · {sizeLabel(file.size)}
        </span>
      </span>
    </span>
  );
}

type Shown =
  { kind: "waiting" } | { kind: "shown"; url: string } | { kind: "unreadable" };

function ImageTile({ file }: { file: AttachmentRef }) {
  const fleet = useFleet();
  const source = useTimelineSource();
  const tile = useRef<HTMLButtonElement>(null);
  const [shown, setShown] = useState<Shown>(() => {
    const url =
      source === null ? null : keptImage(source.node, source.session, file.id);
    return url === null ? { kind: "waiting" } : { kind: "shown", url };
  });
  const [open, setOpen] = useState(false);
  const waiting = shown.kind === "waiting";
  useEffect(() => {
    const element = tile.current;
    if (!waiting || source === null || element === null) {
      return undefined;
    }
    let current = true;
    const stop = whenNear(source.scroller, element, () => {
      void loadImage(
        fleet,
        source.node,
        source.session,
        file.id,
        file.mime,
      ).then((url) => {
        if (current) {
          setShown(
            url === null ? { kind: "unreadable" } : { kind: "shown", url },
          );
        }
      });
    });
    return () => {
      current = false;
      stop();
    };
  }, [fleet, source, file, waiting]);
  return (
    <>
      <button
        ref={tile}
        type="button"
        className="image-tile"
        aria-label={`Open ${file.name}`}
        disabled={shown.kind !== "shown"}
        onClick={() => {
          setOpen(true);
        }}
      >
        {shown.kind === "shown" ? (
          <img src={shown.url} alt="" draggable={false} />
        ) : (
          <span className="image-tile-note">
            {shown.kind === "waiting" ? "" : "Not readable"}
          </span>
        )}
      </button>
      {open && shown.kind === "shown" ? (
        <Dialog
          title={file.name}
          size="lg"
          onClose={() => {
            setOpen(false);
          }}
        >
          <img className="image-full" src={shown.url} alt={file.name} />
        </Dialog>
      ) : null}
    </>
  );
}
