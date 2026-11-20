import { describe, expect, it } from "vitest";

import { fixtureId } from "../../../testing/ids";
import type { ForgeAccess, RepositoryList } from "../../../wire/proto";
import type { Outcome } from "../../outcome";
import {
  machineRepositories,
  repositoryAccessError,
  repositoryAnswer,
} from "./repositories";
import { buildRows } from "./rows";

const LAPTOP = fixtureId("NodeKey", "laptop");
const DESKTOP = fixtureId("NodeKey", "desktop");

const READY: ForgeAccess = { access: "ready", login: "owner" };

const FOUND: Outcome<RepositoryList> = {
  ok: true,
  value: {
    access: READY,
    repositories: [
      {
        path: "owner/vessel",
        description: "Agents on every machine",
        visibility: "private",
        updated_at: "2026-11-19T00:00:00Z",
        clone_url: "git@github.com:owner/vessel.git",
      },
      {
        path: "owner/tools",
        visibility: "public",
        updated_at: "2026-11-18T00:00:00Z",
        clone_url: "https://github.com/owner/tools",
      },
    ],
  },
};

function rowsFor(query: string, answer: Outcome<RepositoryList>) {
  const found = machineRepositories(
    repositoryAnswer(LAPTOP, query, answer),
    LAPTOP,
    query,
    true,
  );
  return buildRows({
    page: {
      kind: "github-search",
      node: LAPTOP,
      query,
      activeIndex: 0,
      error: null,
    },
    machines: [],
    machine: null,
    recentPaths: [],
    machinePaths: [],
    shown: new Map(),
    lastCloneParent: null,
    tilde: true,
    repositories: found.choices,
  });
}

describe("the Clone from GitHub page's search (forge.repos)", () => {
  it("lists the node's rows after the typed ones, titled owner/name", () => {
    const rows = rowsFor("someone/else", FOUND);
    expect(rows.map((row) => row.title)).toEqual([
      "someone/else via HTTPS",
      "someone/else via SSH",
      "owner/vessel",
      "owner/tools",
    ]);
    expect(rows[2]?.subtitle).toBe("Agents on every machine");
    expect(rows[3]?.subtitle).toBeNull();
    const chosen = rows[2]?.action;
    expect(chosen?.kind === "repository" && chosen.repository.cloneUrl).toBe(
      "git@github.com:owner/vessel.git",
    );
  });

  it("drops the typed choices when the search has that repository", () => {
    const rows = rowsFor("Owner/Vessel", FOUND);
    expect(rows.map((row) => row.title)).toEqual([
      "owner/vessel",
      "owner/tools",
    ]);
  });

  it("counts an answer only for its machine and the text as it is", () => {
    const answer = repositoryAnswer(LAPTOP, "ves", FOUND);
    expect(machineRepositories(answer, DESKTOP, "ves", true)).toEqual({
      choices: [],
      loading: true,
      error: null,
    });
    expect(machineRepositories(answer, LAPTOP, "vess", true).loading).toBe(
      true,
    );
    expect(machineRepositories(answer, LAPTOP, "ves", false)).toEqual({
      choices: [],
      loading: false,
      error: null,
    });
  });

  it("says why the node could not search in Paseo's words", () => {
    expect(repositoryAccessError(READY)).toBeNull();
    expect(
      repositoryAccessError({
        access: "signed_out",
        host: "github.com",
        gh: "missing",
      }),
    ).toBe("GitHub CLI (gh) is not installed or not in PATH");
    expect(
      repositoryAccessError({
        access: "signed_out",
        host: "github.com",
        gh: "signed_out",
      }),
    ).toBe(
      "GitHub CLI is not authenticated. Run gh auth login on the machine.",
    );
    expect(
      repositoryAccessError({
        access: "rate_limited",
        host: "github.com",
        until: "2026-11-19T00:00:00Z",
      }),
    ).toBe("GitHub search failed");
    const failed = repositoryAnswer(LAPTOP, "x", {
      ok: false,
      problem: { from: "op", reason: "busy", message: "busy" },
    });
    expect(machineRepositories(failed, LAPTOP, "x", true).error).toBe(
      "Unable to search GitHub repositories",
    );
  });
});
