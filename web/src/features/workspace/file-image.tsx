// The image panel (F-F40), Paseo's `ImagePanel`: Loading until the bytes
// are read, the error's words in the danger ink if they cannot be, else
// the image zoomable. `version` is the file's as the Files tab last read
// it (its watch re-reads the file when the disk changes), so a new image
// on disk is read and shown again.

import "./file-viewers.css";

import type { NodeKey } from "../../wire/bridge";
import type { RelPath, WorkspaceRoot } from "../../wire/proto";
import { FileLoading } from "./file-loading";
import { useImageUrl } from "./use-image-url";
import { ZoomableImage } from "./zoomable-image";

export function FileImage({
  node,
  root,
  path,
  name,
  type,
  version,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  path: RelPath;
  name: string;
  type: string;
  version: string;
}) {
  const url = useImageUrl(node, root, path, type, version);
  switch (url.status) {
    case "loading":
      return <FileLoading />;
    case "error":
      return <div className="file-state error">{url.message}</div>;
    case "ready":
      return <ZoomableImage src={url.url} label={name} />;
  }
}
