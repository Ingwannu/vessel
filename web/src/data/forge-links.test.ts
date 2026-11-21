import { describe, expect, it, vi } from "vitest";

import type { OriginRepo } from "../wire/proto";
import {
  type ForgeId,
  forgeLink,
  forgeOf,
  forgePage,
  type LinkedFile,
  linkedFileOf,
  openForgePage,
  withoutLine,
} from "./forge-links";

// Each remote below is Paseo's (`git/forge-url.test.ts`, pinned 0.15.4) as
// the node sends it: host and path only (`vessel_forge::remote::web`).
type Remote = Pick<OriginRepo, "host" | "path">;

function remote(host: string, path: string): Remote {
  return { host, path };
}

const ACME = remote("github.com", "acme/repo");

function url(
  forge: ForgeId,
  at: Remote,
  branch: string,
  file: LinkedFile | null = null,
): string | null {
  return forgePage(forge, at, branch, file)?.url ?? null;
}

describe("the branch's page (Paseo's buildForgeBranchTreeUrl)", () => {
  it("builds a branch-specific GitHub tree URL", () => {
    expect(url("github", ACME, "feature/workspace-button")).toBe(
      "https://github.com/acme/repo/tree/feature/workspace-button",
    );
  });

  it("encodes reserved branch characters while preserving slash-separated branch names", () => {
    expect(url("github", ACME, "feature/ship #42")).toBe(
      "https://github.com/acme/repo/tree/feature/ship%20%2342",
    );
  });

  it("uses the GitLab /-/tree/ infix and supports subgroups", () => {
    expect(url("gitlab", remote("gitlab.com", "group/sub/repo"), "main")).toBe(
      "https://gitlab.com/group/sub/repo/-/tree/main",
    );
  });

  it("uses the Gitea-family /src/branch/ infix", () => {
    expect(url("gitea", remote("gitea.com", "acme/repo"), "main")).toBe(
      "https://gitea.com/acme/repo/src/branch/main",
    );
    expect(url("codeberg", remote("codeberg.org", "acme/repo"), "main")).toBe(
      "https://codeberg.org/acme/repo/src/branch/main",
    );
  });

  it("preserves a non-default port for a self-hosted https remote", () => {
    expect(
      url(
        "forgejo",
        remote("home-git.example.com:60443", "team/repo"),
        "master",
      ),
    ).toBe("https://home-git.example.com:60443/team/repo/src/branch/master");
  });

  it("omits the port for a self-hosted remote on the default port", () => {
    expect(
      url("forgejo", remote("home-git.example.com", "team/repo"), "master"),
    ).toBe("https://home-git.example.com/team/repo/src/branch/master");
  });

  it("returns null when the current branch is unavailable", () => {
    expect(url("github", ACME, "HEAD")).toBeNull();
    expect(forgePage("github", ACME, null, null)).toBeNull();
    expect(forgePage("github", ACME, "  ", null)).toBeNull();
  });
});

describe("a file's page (Paseo's buildForgeBlobUrl)", () => {
  const file = (
    path: string,
    lineStart?: number,
    lineEnd?: number,
  ): LinkedFile => ({
    path,
    ...(lineStart === undefined ? {} : { lineStart }),
    ...(lineEnd === undefined ? {} : { lineEnd }),
  });

  it("builds a blob URL for a file path", () => {
    expect(url("github", ACME, "main", file("src/index.ts"))).toBe(
      "https://github.com/acme/repo/blob/main/src/index.ts",
    );
  });

  it("appends a single-line anchor", () => {
    expect(url("github", ACME, "main", file("src/index.ts", 12))).toBe(
      "https://github.com/acme/repo/blob/main/src/index.ts#L12",
    );
  });

  it("appends a GitHub line range anchor (#L12-L20)", () => {
    expect(url("github", ACME, "main", file("src/index.ts", 12, 20))).toBe(
      "https://github.com/acme/repo/blob/main/src/index.ts#L12-L20",
    );
  });

  it("uses the GitLab /-/blob/ infix and #L12-20 range anchor", () => {
    expect(
      url(
        "gitlab",
        remote("gitlab.com", "group/sub/repo"),
        "main",
        file("src/index.ts", 12, 20),
      ),
    ).toBe("https://gitlab.com/group/sub/repo/-/blob/main/src/index.ts#L12-20");
  });

  it("uses the Gitea-family /src/branch/ blob path with a #L12-L20 anchor", () => {
    expect(
      url(
        "forgejo",
        remote("codeberg.org", "acme/repo"),
        "main",
        file("src/index.ts", 12, 20),
      ),
    ).toBe(
      "https://codeberg.org/acme/repo/src/branch/main/src/index.ts#L12-L20",
    );
  });

  it("derives the web host from a self-hosted remote (GitHub Enterprise)", () => {
    expect(
      url(
        "github",
        remote("github.acme.internal", "team/repo"),
        "main",
        file("src/index.ts"),
      ),
    ).toBe("https://github.acme.internal/team/repo/blob/main/src/index.ts");
  });

  it("preserves a non-default port for a self-hosted https remote", () => {
    expect(
      url(
        "forgejo",
        remote("home-git.example.com:60443", "team/repo"),
        "master",
        file("src/index.ts", 12, 20),
      ),
    ).toBe(
      "https://home-git.example.com:60443/team/repo/src/branch/master/src/index.ts#L12-L20",
    );
  });

  it("canonicalizes the github.com SSH-alias host to the web host", () => {
    expect(
      url(
        "github",
        remote("ssh.github.com", "acme/repo"),
        "main",
        file("src/index.ts"),
      ),
    ).toBe("https://github.com/acme/repo/blob/main/src/index.ts");
  });

  it("strips leading slashes and encodes path segments", () => {
    expect(url("github", ACME, "main", file("/src/a b/c#d.ts"))).toBe(
      "https://github.com/acme/repo/blob/main/src/a%20b/c%23d.ts",
    );
  });

  it("normalizes harmless dot segments in the blob path", () => {
    expect(url("github", ACME, "main", file("./src/../index.ts"))).toBe(
      "https://github.com/acme/repo/blob/main/index.ts",
    );
  });

  it("returns null for blob paths that escape above the repo root", () => {
    expect(url("github", ACME, "main", file("../outside.ts"))).toBeNull();
  });

  it("returns null when the path is missing", () => {
    expect(url("github", ACME, "main", file(""))).toBeNull();
  });
});

describe("which forge a status's origin is on (Paseo's forge-manifest.ts)", () => {
  const origin = (host: string, kind?: OriginRepo["kind"]): OriginRepo => ({
    ...remote(host, "acme/repo"),
    ...(kind === undefined ? {} : { kind }),
  });

  it("maps each forge's public hosts", () => {
    expect(forgeOf(origin("github.com"))).toBe("github");
    expect(forgeOf(origin("ssh.github.com"))).toBe("github");
    expect(forgeOf(origin("gitlab.com"))).toBe("gitlab");
    expect(forgeOf(origin("gitea.com"))).toBe("gitea");
    expect(forgeOf(origin("codeberg.org"))).toBe("codeberg");
  });

  it("takes the node's word for a self-hosted GitHub", () => {
    expect(forgeOf(origin("github.acme.internal"))).toBeNull();
    expect(forgeOf(origin("github.acme.internal", "github"))).toBe("github");
    // A kind a newer node knows falls back to the public hosts.
    expect(forgeOf(origin("gitlab.com", "unknown"))).toBe("gitlab");
  });

  it("links nothing for a host no forge owns (Paseo: no grammar)", () => {
    expect(forgeOf(origin("bitbucket.org"))).toBeNull();
    expect(forgeOf(origin("gitlab.example.com"))).toBeNull();
    // Paseo's two no-grammar cases: Bitbucket's branch and file.
    expect(forgeLink(origin("bitbucket.org"), "main", null)).toBeNull();
    expect(
      forgeLink(origin("bitbucket.org"), "main", { path: "src/index.ts" }),
    ).toBeNull();
    expect(forgeLink(null, "main", null)).toBeNull();
    expect(forgeLink(undefined, "main", null)).toBeNull();
  });

  it("names the forge it links to", () => {
    expect(forgeLink(origin("gitlab.com"), "main", null)).toEqual({
      forge: "gitlab",
      name: "GitLab",
      url: "https://gitlab.com/acme/repo/-/tree/main",
    });
  });
});

describe("opening a forge page", () => {
  it("opens https only, in a new isolated tab", () => {
    const open = vi.fn<(url: string) => void>();
    expect(openForgePage("https://github.com/acme/repo", open)).toBe(true);
    expect(open).toHaveBeenCalledWith("https://github.com/acme/repo");
    for (const refused of [
      "http://github.com/acme/repo",
      "javascript:alert(1)",
      "//github.com/acme/repo",
      "not a url",
    ]) {
      expect(openForgePage(refused, open)).toBe(false);
    }
    expect(open).toHaveBeenCalledTimes(1);
  });
});

describe("the file a link names", () => {
  it("is the active file tab's, at the line it was opened at", () => {
    const lines = new Map([["src/a.ts", { line: 7, seq: 1 }]]);
    expect(linkedFileOf({ kind: "file", path: "src/a.ts" }, lines)).toEqual({
      path: "src/a.ts",
      lineStart: 7,
    });
    expect(linkedFileOf({ kind: "file", path: "src/b.ts" }, lines)).toEqual({
      path: "src/b.ts",
    });
    expect(linkedFileOf({ kind: "files" }, lines)).toBeNull();
    expect(linkedFileOf(undefined, lines)).toBeNull();
  });
});

describe("a file opened again at no line", () => {
  it("drops its line, so the link names none", () => {
    const lines = new Map([["src/a.ts", { line: 7, seq: 1 }]]);
    const after = withoutLine(lines, "src/a.ts");
    expect(linkedFileOf({ kind: "file", path: "src/a.ts" }, after)).toEqual({
      path: "src/a.ts",
    });
    expect(withoutLine(after, "src/a.ts")).toBe(after);
  });
});

describe("a repository path the page would not link", () => {
  it("links nothing for an empty segment or a climb", () => {
    for (const path of ["acme//repo", "acme/../repo", "/acme/repo", ""]) {
      expect(
        forgePage("github", { host: "github.com", path }, "main", null),
      ).toBeNull();
    }
  });
});
