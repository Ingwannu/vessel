// A file's Preview in the Files tab, Paseo's built-in panels
// (`file-panels/builtin.tsx`, pinned 0.15.4):
// - Markdown (F-F38): the front matter as a key/value table above the
//   body, the body through the conversation's Markdown renderer; a
//   relative link opens that file here, an image is read through the node
//   (`markdown-paths.ts` resolves both from the Markdown file's folder);
// - HTML (F-F39): the file in a sandboxed `srcdoc` frame under Paseo's
//   preview policy. The coordinator's ruling: no `allow-scripts` here, and
//   the frame inherits the page's own policy, so the file's scripts do not
//   run (plan difference 1; TRACKER 814 is the separate preview origin
//   that would run them). Its layout, inline styles and `data:` images
//   show.
// Both draw the text they are given: the file as read, or the edit open.

import "./file-viewers.css";

import { type ReactNode, useMemo } from "react";

import { Markdown, type MarkdownFiles } from "../../data/markdown";
import { imageType, type PreviewPanel } from "../../data/workspace/file-panels";
import { markdownDocument } from "../../data/workspace/front-matter";
import { withPreviewCsp } from "../../data/workspace/html-preview";
import { markdownTarget } from "../../data/workspace/markdown-paths";
import type { NodeKey } from "../../wire/bridge";
import type { RelPath, WorkspaceRoot } from "../../wire/proto";
import { useImageUrl } from "./use-image-url";

export interface PreviewProps {
  panel: PreviewPanel;
  node: NodeKey;
  root: WorkspaceRoot;
  /** The root's folder on the node, when known: an absolute link inside
   * it is a file of the workspace. */
  folder: string | null;
  path: RelPath;
  text: string;
  /** The file's version, so its images are read again when it changes. */
  version: string;
  onOpenFile: (path: RelPath) => void;
}

export function FilePreview(props: PreviewProps) {
  return props.panel === "html" ? (
    <HtmlPreview text={props.text} />
  ) : (
    <MarkdownPreview {...props} />
  );
}

function HtmlPreview({ text }: { text: string }) {
  const document = useMemo(() => withPreviewCsp(text), [text]);
  return (
    <iframe
      className="html-frame"
      aria-label="HTML preview"
      data-testid="file-html-preview"
      srcDoc={document}
      // No tokens at all: no script, no forms, no popups, an opaque
      // origin that cannot reach this page.
      sandbox=""
      referrerPolicy="no-referrer"
    />
  );
}

function MarkdownPreview({
  node,
  root,
  folder,
  path,
  text,
  version,
  onOpenFile,
}: PreviewProps) {
  const document = useMemo(() => markdownDocument(text), [text]);
  const files = useMemo<MarkdownFiles>(
    () => ({
      link: (href: string, label: ReactNode) => {
        const target = markdownTarget(href, path, folder);
        return target.kind === "file" ? (
          <button
            type="button"
            className="md-file-link"
            onClick={() => {
              onOpenFile(target.path);
            }}
          >
            {label}
          </button>
        ) : (
          label
        );
      },
      image: (src: string, alt: string) => {
        const target = markdownTarget(src, path, folder);
        switch (target.kind) {
          case "data":
            return <img src={target.url} alt={alt} />;
          case "file": {
            const type = imageType(target.path);
            return type === null ? (
              alt
            ) : (
              <WorkspaceImage
                node={node}
                root={root}
                path={target.path}
                type={type}
                version={version}
                alt={alt}
              />
            );
          }
          case "web":
            // The page's policy loads no image from the web (plan
            // difference 7): its words, as a link to it.
            return (
              <a href={target.url} target="_blank" rel="noopener noreferrer">
                {alt === "" ? target.url : alt}
              </a>
            );
          case "none":
            return alt;
        }
      },
    }),
    [node, root, folder, path, version, onOpenFile],
  );
  return (
    <div className="md-doc-scroll">
      <div className="md-doc" data-testid="markdown-preview">
        {document.frontMatter.length > 0 ? (
          <dl className="md-front" data-testid="markdown-front-matter">
            {document.frontMatter.map((row) => (
              <FrontMatterRow key={row.key} name={row.key} value={row.value} />
            ))}
          </dl>
        ) : null}
        <Markdown files={files}>{document.body}</Markdown>
      </div>
    </div>
  );
}

function FrontMatterRow({ name, value }: { name: string; value: string }) {
  return (
    <>
      <dt>{name}</dt>
      <dd>{value}</dd>
    </>
  );
}

function WorkspaceImage({
  node,
  root,
  path,
  type,
  version,
  alt,
}: {
  node: NodeKey;
  root: WorkspaceRoot;
  path: RelPath;
  type: string;
  version: string;
  alt: string;
}) {
  const url = useImageUrl(node, root, path, type, version);
  return url.status === "ready" ? <img src={url.url} alt={alt} /> : alt;
}
