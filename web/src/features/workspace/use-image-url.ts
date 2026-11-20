// A workspace image as an address the page may show: its bytes read
// through the node (`data/workspace/file-bytes.ts`) and handed back as a
// `data:` URL (plan difference 3: the page's policy allows `data:`
// images, not `blob:` ones, and the page has no HTTP route to the file).
// The image panel and the Markdown preview's images read through it;
// `version` changes when the file does, and the image is read again.

import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { dataUrl, readFileBytes } from "../../data/workspace/file-bytes";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, WorkspaceRoot } from "../../wire/proto";

/** Paseo's `useFileUrl` states. */
export type ImageUrl =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; url: string };

interface Answer {
  path: RelPath;
  url: ImageUrl;
}

export function useImageUrl(
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  type: string,
  version: string,
): ImageUrl {
  const fleet = useFleet();
  const [answer, setAnswer] = useState<Answer | null>(null);
  useEffect(() => {
    let current = true;
    void readFileBytes(fleet, node, root, path).then((bytes) => {
      if (!current) return;
      setAnswer({
        path,
        url: bytes.ok
          ? { status: "ready", url: dataUrl(bytes.value.pages, type) }
          : { status: "error", message: bytes.problem.message },
      });
    });
    return () => {
      current = false;
    };
    // `version` is the file's: a new one reads the bytes again.
  }, [fleet, node, root, path, type, version]);
  // Another file shows Loading until its own answer; a new version of this
  // one keeps the image shown until the new bytes arrive, as the text view
  // keeps its lines.
  return answer?.path === path ? answer.url : { status: "loading" };
}
