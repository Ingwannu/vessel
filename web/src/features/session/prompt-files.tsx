// The files a prompt carried, over its text in the timeline
// (agent-orchestrator's attachment row above the message): images as
// 64px-tall tiles that open full size, other files as small cards with
// their name, type and size. An image is fetched only when its row comes
// near the view (data/images.ts); one sent from this tab is shown from
// its local file. The tile shows a small `data:` thumbnail; the image at
// full size is read into a `data:` address only when it is opened (the
// page's content policy takes no `blob:` address).

import "./prompt-files.css";

import { useEffect, useRef, useState } from "react";

import { useFleet } from "../../context";
import { sizeLabel } from "../../data/attach";
import { type KeptImage, keptImage, loadImage } from "../../data/images";
import { dataAddress } from "../../data/thumbnail";
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
  | { kind: "waiting" }
  | { kind: "shown"; image: KeptImage; thumb: string }
  | { kind: "unreadable" };

function shownOf(image: KeptImage | null): Shown {
  if (image === null) {
    return { kind: "unreadable" };
  }
  return image.thumb === null
    ? { kind: "unreadable" }
    : { kind: "shown", image, thumb: image.thumb };
}

function ImageTile({ file }: { file: AttachmentRef }) {
  const fleet = useFleet();
  const source = useTimelineSource();
  const tile = useRef<HTMLButtonElement>(null);
  const [shown, setShown] = useState<Shown>(() => {
    const image =
      source === null ? null : keptImage(source.node, source.session, file.id);
    return image === null ? { kind: "waiting" } : shownOf(image);
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
      ).then((image) => {
        if (current) {
          setShown(shownOf(image));
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
          <img src={shown.thumb} alt="" draggable={false} />
        ) : (
          <span className="image-tile-note">
            {shown.kind === "waiting" ? "" : "Not readable"}
          </span>
        )}
      </button>
      {open && shown.kind === "shown" ? (
        <FullImage
          name={file.name}
          image={shown.image}
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </>
  );
}

/** The image at full size, read from its bytes when opened. */
function FullImage({
  name,
  image,
  onClose,
}: {
  name: string;
  image: KeptImage;
  onClose: () => void;
}) {
  const [address, setAddress] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void dataAddress(image.blob).then((read) => {
      if (current) {
        setAddress(read);
      }
    });
    return () => {
      current = false;
    };
  }, [image]);
  return (
    <Dialog title={name} size="lg" onClose={onClose}>
      {address === null ? (
        <span className="image-full-note">Reading the image</span>
      ) : (
        <img className="image-full" src={address} alt={name} />
      )}
    </Dialog>
  );
}
