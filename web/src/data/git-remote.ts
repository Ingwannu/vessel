// Git remote addresses, Paseo's `protocol/src/git-remote.ts` (pinned
// 0.15.4): which text is a complete remote (a URL or the scp form) and
// which GitHub repository it names. Add project's manual clone rows use
// it (`data/workspace/add-project/options.ts`).
//
// [Decision Log]
// - Purpose: "Clone from GitHub" before its search (TRACKER 440) offers
//   Paseo's manual rows: a typed full remote, or `owner/repo` via HTTPS
//   or SSH.
// - Constraint: the node's `CloneUrl` takes the `https`, `http`, `ssh`,
//   `git` transports and the scp form; Paseo's parser takes `https`,
//   `http`, `ssh` and scp, so whatever this accepts the node accepts.
// - Choice: Paseo's parser as it is, GitHub's cloud hosts from its forge
//   manifest (`github.com`, `ssh.github.com`).
// - Cost: a `git://` remote is not offered as a manual row (as in Paseo).

const GITHUB_HOSTS = new Set(["github.com", "ssh.github.com"]);

const TRANSPORTS: Readonly<Record<string, GitRemoteLocation["transport"]>> = {
  "https:": "https",
  "http:": "http",
  "ssh:": "ssh",
};

const DEFAULT_PORTS: Readonly<Record<string, string>> = {
  "https:": "443",
  "http:": "80",
  "ssh:": "22",
};

/** Where a remote points: its transport, host, non-default port, path. */
export interface GitRemoteLocation {
  transport: "scp" | "ssh" | "http" | "https";
  host: string;
  port?: string;
  path: string;
}

/** A GitHub repository a remote names. */
export interface GitHubRemoteIdentity {
  owner: string;
  name: string;
  repo: string;
}

const SCP = /^[^@]+@([^:]+):(.+)$/u;
const HOST = /^[a-z0-9](?:[a-z0-9._-]*[a-z0-9])?$/u;

function normalizeHost(host: string): string {
  return host.trim().replace(/\.+$/u, "").toLowerCase();
}

function normalizeRemotePath(path: string): string | null {
  let normalized = path.trim().replace(/^\/+|\/+$/gu, "");
  if (normalized.endsWith(".git")) normalized = normalized.slice(0, -4);
  return normalized === "" ? null : normalized;
}

/** `remoteUrl` as a remote's parts; `null` when it is not a complete
 * remote. */
export function parseGitRemoteLocation(
  remoteUrl: string,
): GitRemoteLocation | null {
  const trimmed = remoteUrl.trim();
  if (trimmed === "") return null;
  // The scp form has no scheme; tested first it would read
  // `ssh://git@host:22/x` as a login `ssh://git`.
  const scp = trimmed.includes("://") ? null : SCP.exec(trimmed);
  if (scp !== null) {
    const host = normalizeHost(scp[1] ?? "");
    const path = normalizeRemotePath(scp[2] ?? "");
    if (!HOST.test(host) || path === null) return null;
    return { transport: "scp", host, path };
  }
  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return null;
  }
  const protocol = parsed.protocol.toLowerCase();
  const transport = TRANSPORTS[protocol];
  if (transport === undefined) return null;
  const host = normalizeHost(parsed.hostname);
  let decoded: string;
  try {
    decoded = decodeURIComponent(parsed.pathname);
  } catch {
    return null;
  }
  const path = normalizeRemotePath(decoded);
  if (!HOST.test(host) || path === null) return null;
  const port =
    parsed.port !== "" && parsed.port !== DEFAULT_PORTS[protocol]
      ? parsed.port
      : undefined;
  return port === undefined
    ? { transport, host, path }
    : { transport, host, port, path };
}

/** Whether `repo` is a complete remote rather than `owner/repo`. */
export function isCompleteGitRemote(repo: string): boolean {
  return parseGitRemoteLocation(repo) !== null;
}

function parseGitHubRemoteIdentity(path: string): GitHubRemoteIdentity | null {
  const segments = path.split("/").filter((segment) => segment !== "");
  if (segments.length !== 2) return null;
  const [owner, name] = segments;
  if (owner === undefined || name === undefined) return null;
  return { owner, name, repo: `${owner}/${name}` };
}

/** The GitHub repository a remote names, when it is on GitHub. */
export function parseGitHubRemoteUrl(
  remoteUrl: string,
): GitHubRemoteIdentity | null {
  const location = parseGitRemoteLocation(remoteUrl);
  if (location === null || !GITHUB_HOSTS.has(location.host)) return null;
  return parseGitHubRemoteIdentity(location.path);
}
