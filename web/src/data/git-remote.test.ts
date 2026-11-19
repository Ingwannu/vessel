import { describe, expect, it } from "vitest";

import {
  isCompleteGitRemote,
  parseGitHubRemoteUrl,
  parseGitRemoteLocation,
} from "./git-remote";

// Paseo's `protocol/src/git-remote.test.ts` (pinned 0.15.4).
describe("isCompleteGitRemote", () => {
  it("treats supported URLs and scp-like addresses as complete remotes", () => {
    for (const repo of [
      "https://github.com/owner/repo",
      "http://internal/owner/repo.git",
      "ssh://git@github.com/owner/repo",
      "git@github.com:owner/repo.git",
      "  https://github.com/owner/repo  ",
    ]) {
      expect(isCompleteGitRemote(repo)).toBe(true);
    }
  });

  it("treats owner/repo shorthand as incomplete (needs a clone protocol)", () => {
    expect(isCompleteGitRemote("owner/repo")).toBe(false);
    expect(isCompleteGitRemote("owner/repo.git")).toBe(false);
    expect(isCompleteGitRemote("")).toBe(false);
  });

  it("rejects schemes the parser does not accept", () => {
    for (const repo of [
      "git://github.com/owner/repo",
      "ftp://host/repo",
      "file:///tmp/repo",
    ]) {
      expect(isCompleteGitRemote(repo)).toBe(false);
      expect(parseGitRemoteLocation(repo)).toBeNull();
    }
  });
});

describe("parseGitRemoteLocation port", () => {
  it("preserves an explicit non-default port", () => {
    expect(
      parseGitRemoteLocation("https://home-git.example.com:60443/team/repo.git")
        ?.port,
    ).toBe("60443");
    expect(
      parseGitRemoteLocation("http://internal.example.com:3000/team/repo.git")
        ?.port,
    ).toBe("3000");
  });

  it("omits the port for a default-port or scp remote", () => {
    expect(
      parseGitRemoteLocation("https://github.com/acme/repo.git")?.port,
    ).toBeUndefined();
    expect(
      parseGitRemoteLocation("git@host.example.com:team/repo.git")?.port,
    ).toBeUndefined();
    expect(
      parseGitRemoteLocation("ssh://git@github.com:22/acme/repo.git"),
    ).toEqual(parseGitRemoteLocation("ssh://git@github.com/acme/repo.git"));
  });

  it("reads a scheme remote as a URL rather than scp form", () => {
    expect(
      parseGitRemoteLocation("ssh://git@example.com:2222/acme/repo.git"),
    ).toEqual({
      transport: "ssh",
      host: "example.com",
      port: "2222",
      path: "acme/repo",
    });
  });
});

describe("parseGitHubRemoteUrl", () => {
  it("names the repository of a GitHub remote only", () => {
    expect(parseGitHubRemoteUrl("git@github.com:getpaseo/paseo.git")).toEqual({
      owner: "getpaseo",
      name: "paseo",
      repo: "getpaseo/paseo",
    });
    expect(parseGitHubRemoteUrl("https://gitlab.com/a/b.git")).toBeNull();
    expect(parseGitHubRemoteUrl("https://github.com/a/b/c")).toBeNull();
  });
});
