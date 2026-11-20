// "Open on <forge>" (TRACKER 467 step B, ADR 0123): the web page of a
// workspace's branch, or of the shown file at its line, on the forge
// `origin` names. Paseo's rules (pinned 0.15.4): the forge
// (`git/forge.ts:49`, `protocol/forge-manifest.ts:59-111`), the web
// location (`git/forge-url.ts:69`), the branch tree (`:124`), the file
// (`:137`, path `:102`) and each forge's infixes and line anchor
// (`git/forges/github.ts:85`, `gitlab.ts:23,163`,
// `git/client-forge-module.ts:47-60`).
//
// [Decision Log]
// - Purpose: Paseo's open-in-editor list ends with the forge's page; in a
//   browser (no desktop bridge) it is the only target the header has.
// - Constraint: the node sends `origin`'s host and path only, never its
//   address (which may hold a token), and `kind` only for a host it alone
//   knows (a self-hosted GitHub it holds a token for); the public hosts of
//   every forge are mapped here, as Paseo's client maps them.
// - Choice: one table of Paseo's five forges, in its order; a link is
//   always `https://`, and `openForgePage` opens nothing else (ADR 0078's
//   opener rule, the coordinator's condition on step B).
// - Cost: the same rules in Rust for the native app (its view crate);
//   both run Paseo's `forge-url.test.ts` cases.

import type { OriginRepo } from "../wire/proto";

/** A forge Paseo links to. */
export type ForgeId = "github" | "gitlab" | "gitea" | "forgejo" | "codeberg";

interface Forge {
  id: ForgeId;
  /** The brand's name ("Open on GitHub"). */
  name: string;
  /** Its public hosts, the web host first (`forge-manifest.ts`). */
  cloudHosts: readonly string[];
  treeInfix: string;
  blobInfix: string;
  anchor: (start: number, end?: number) => string;
}

/** `#L1` or `#L1-L3` (GitHub and the Gitea family). */
function githubAnchor(start: number, end?: number): string {
  return end !== undefined && end > start ? `#L${start}-L${end}` : `#L${start}`;
}

/** `#L1` or `#L1-3` (GitLab). */
function gitlabAnchor(start: number, end?: number): string {
  return end !== undefined && end > start ? `#L${start}-${end}` : `#L${start}`;
}

const GITEA_FAMILY = {
  treeInfix: "/src/branch/",
  blobInfix: "/src/branch/",
  anchor: githubAnchor,
} as const;

const FORGES: readonly Forge[] = [
  {
    id: "github",
    name: "GitHub",
    cloudHosts: ["github.com", "ssh.github.com"],
    treeInfix: "/tree/",
    blobInfix: "/blob/",
    anchor: githubAnchor,
  },
  {
    id: "gitlab",
    name: "GitLab",
    cloudHosts: ["gitlab.com"],
    treeInfix: "/-/tree/",
    blobInfix: "/-/blob/",
    anchor: gitlabAnchor,
  },
  { id: "gitea", name: "Gitea", cloudHosts: ["gitea.com"], ...GITEA_FAMILY },
  // Forgejo has no public host: Paseo finds it only by asking `tea`,
  // which the node does not (ADR 0123 "Step B").
  { id: "forgejo", name: "Forgejo", cloudHosts: [], ...GITEA_FAMILY },
  {
    id: "codeberg",
    name: "Codeberg",
    cloudHosts: ["codeberg.org"],
    ...GITEA_FAMILY,
  },
];

function forgeById(id: ForgeId): Forge {
  const forge = FORGES.find((entry) => entry.id === id);
  if (forge === undefined) {
    throw new Error(`no forge ${id}`);
  }
  return forge;
}

/** `host` without the port the node keeps for an `http(s)` address. */
function hostName(host: string): string {
  const colon = host.lastIndexOf(":");
  return colon === -1 ? host : host.slice(0, colon);
}

/** The forge `origin` is on: the node's word for a host it alone knows,
 * else the public host's (Paseo: `resolvedForge ?? forgeFromRemoteUrl`). */
export function forgeOf(origin: OriginRepo): ForgeId | null {
  if (origin.kind === "github") {
    return "github";
  }
  const name = hostName(origin.host);
  return (
    FORGES.find((forge) => forge.cloudHosts.includes(name))?.id ?? null
  );
}

/** The forge's brand name. */
export function forgeName(id: ForgeId): string {
  return forgeById(id).name;
}

/** Host and port for a page: a public host's web host (an SSH alias such
 * as `ssh.github.com` is GitHub's `github.com`), else the host as the
 * node names it, with its `http(s)` port (`forge-url.ts:69`). */
function authority(forge: Forge, host: string): string {
  return forge.cloudHosts.includes(hostName(host))
    ? (forge.cloudHosts[0] ?? host)
    : host;
}

function encodeSegments(text: string): string {
  return text.split("/").map(encodeURIComponent).join("/");
}

/** A file's path within the repository, or null when it climbs out of it
 * (`forge-url.ts:102`). */
function blobPath(path: string): string | null {
  const segments: string[] = [];
  const trimmed = path.trim().replace(/\\/g, "/").replace(/^\/+/, "");
  for (const segment of trimmed.split("/")) {
    if (segment === "" || segment === ".") {
      continue;
    }
    if (segment === "..") {
      if (segments.length === 0) {
        return null;
      }
      segments.pop();
      continue;
    }
    segments.push(segment);
  }
  return segments.length === 0 ? null : segments.join("/");
}

/** A file to link and the line its tab was opened at. */
export interface LinkedFile {
  path: string;
  lineStart?: number;
  lineEnd?: number;
}

/** A forge page to open. */
export interface ForgeLink {
  forge: ForgeId;
  name: string;
  url: string;
}

/** The page of `branch` on `origin`'s forge, or of `file` on it; null with
 * no forge, no branch (a detached `HEAD`), or a file outside the
 * repository (`planner.ts:147`). */
export function forgeLink(
  origin: OriginRepo | null | undefined,
  branch: string | null | undefined,
  file: LinkedFile | null,
): ForgeLink | null {
  if (origin === null || origin === undefined) {
    return null;
  }
  const id = forgeOf(origin);
  return id === null ? null : forgePage(id, origin, branch, file);
}

/** The page on forge `id` (Paseo's `buildForgeBranchTreeUrl` and
 * `buildForgeBlobUrl`, which take the forge as given). */
export function forgePage(
  id: ForgeId,
  origin: Pick<OriginRepo, "host" | "path">,
  branch: string | null | undefined,
  file: LinkedFile | null,
): ForgeLink | null {
  const trimmed = branch?.trim() ?? "";
  if (trimmed === "" || trimmed === "HEAD") {
    return null;
  }
  const forge = forgeById(id);
  const base = `https://${authority(forge, origin.host)}/${origin.path}`;
  const ref = encodeSegments(trimmed);
  if (file === null) {
    return {
      forge: id,
      name: forge.name,
      url: `${base}${forge.treeInfix}${ref}`,
    };
  }
  const path = blobPath(file.path);
  if (path === null) {
    return null;
  }
  const anchor =
    file.lineStart !== undefined && file.lineStart > 0
      ? forge.anchor(file.lineStart, file.lineEnd)
      : "";
  return {
    forge: id,
    name: forge.name,
    url: `${base}${forge.blobInfix}${ref}/${encodeSegments(path)}${anchor}`,
  };
}

/** Open a forge page in a new, isolated tab; anything but `https://` is
 * refused (ADR 0078). Whether it was opened. */
export function openForgePage(
  url: string,
  open: (url: string) => void = (page) => {
    window.open(page, "_blank", "noopener,noreferrer");
  },
): boolean {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return false;
  }
  if (parsed.protocol !== "https:" || !url.startsWith("https://")) {
    return false;
  }
  open(url);
  return true;
}
