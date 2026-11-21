// Agent prose is untrusted. Only absolute web URLs become links; opening
// a new tab keeps the authenticated UI in place and isolates the opener.
//
// The Files tab's Markdown preview (F-F38) passes `files`: there a link
// that is not a web address may name a file of the workspace, and an
// image is read through the node, both resolved by the caller. Without
// `files` (the conversation) neither is followed.
import type { ReactNode } from "react";
import ReactMarkdown, { defaultUrlTransform } from "react-markdown";

import { isWebUrl } from "./links";

/** How a document read from the workspace shows what it points at. */
export interface MarkdownFiles {
  /** A link that is not a web address: its element. */
  link: (href: string, label: ReactNode) => ReactNode;
  /** An image: its element. */
  image: (src: string, alt: string) => ReactNode;
}

/** The default sanitizer, but an image's own `data:` URL kept (Paseo's
 * preview shows one; the page's policy allows `data:` images). */
function documentUrlTransform(url: string, key: string): string {
  return key === "src" && /^data:image\//iu.test(url)
    ? url
    : defaultUrlTransform(url);
}

export function Markdown({
  children,
  files,
  httpsOnly = false,
}: {
  children: string;
  files?: MarkdownFiles;
  /** Only `https:` addresses become links (a forge's comments, TRACKER
   * 430: a reviewer's text is followed only over a secure link). */
  httpsOnly?: boolean;
}) {
  return (
    <ReactMarkdown
      skipHtml
      urlTransform={
        files === undefined ? defaultUrlTransform : documentUrlTransform
      }
      components={{
        a: ({ href, children: label }) =>
          href !== undefined &&
          isWebUrl(href) &&
          (!httpsOnly || /^https:\/\//iu.test(href)) ? (
            <a href={href} target="_blank" rel="noopener noreferrer">
              {label}
            </a>
          ) : href !== undefined && files !== undefined ? (
            files.link(href, label)
          ) : (
            <>{label}</>
          ),
        ...(files === undefined
          ? {}
          : {
              img: ({ src, alt }) =>
                files.image(typeof src === "string" ? src : "", alt ?? ""),
            }),
      }}
    >
      {children}
    </ReactMarkdown>
  );
}
