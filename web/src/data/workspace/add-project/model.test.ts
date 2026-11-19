import { describe, expect, it } from "vitest";

import { fixtureId } from "../../../testing/ids";
import {
  moveAddProjectActiveIndex,
  moveAddProjectSelection,
  setAddProjectActiveIndex,
  setAddProjectPageInput,
  setAddProjectPageStatus,
  setNewDirectoryName,
} from "./editing";
import {
  type AddProjectMachine,
  applyAvailableMachines,
  backAddProjectPage,
  chooseAddProjectMachine,
  currentAddProjectPage,
  openAddProjectFlow,
  openDirectorySearchPage,
  openGithubLocationPage,
  openNewDirectoryNamePage,
  openNewDirectoryParentPage,
} from "./model";
import {
  addProjectMethodEmptyText,
  buildAddProjectMethods,
  buildCloneLocationOptions,
  buildManualGithubRepositoryChoices,
  buildSuggestedParentDirectories,
  cloneRemote,
  filterAddProjectMachines,
} from "./options";

const MACHINE: AddProjectMachine = {
  node: fixtureId("NodeKey", "host-1"),
  label: "Local",
  canAddProject: true,
  canBrowse: true,
  canCloneGithubRepositories: true,
  canSearchGithubRepositories: true,
  canCreateDirectory: true,
};

const SECOND: AddProjectMachine = {
  ...MACHINE,
  node: fixtureId("NodeKey", "host-2"),
  label: "Remote",
};

const REPOSITORY = {
  id: "repo-1",
  nameWithOwner: "getpaseo/paseo",
  cloneUrl: "git@github.com:getpaseo/paseo.git",
  description: null,
  updatedAt: null,
};

// Paseo's `add-project-flow/model.test.ts` (pinned 0.15.4), host as
// machine; then Vessel's own cases.
describe("Add Project navigation", () => {
  it("skips a single connected machine without adding it to history", () => {
    const state = openAddProjectFlow({ machines: [MACHINE] });
    expect(currentAddProjectPage(state)).toEqual({
      kind: "method",
      node: MACHINE.node,
      activeIndex: 0,
      error: null,
      isSubmitting: false,
    });
    expect(backAddProjectPage(state)).toBeNull();
  });

  it("starts on the preferred machine, else on the machine page", () => {
    const preferred = openAddProjectFlow({
      machines: [MACHINE, SECOND],
      preferred: SECOND.node,
    });
    expect(currentAddProjectPage(preferred)).toMatchObject({
      kind: "method",
      node: SECOND.node,
    });
    const neither = openAddProjectFlow({ machines: [MACHINE, SECOND] });
    expect(currentAddProjectPage(neither).kind).toBe("host");
  });

  it("skips the machine page when the only machine arrives late", () => {
    let state = openAddProjectFlow({ machines: [] });
    state = applyAvailableMachines(state, [MACHINE]);
    expect(currentAddProjectPage(state)).toMatchObject({
      kind: "method",
      node: MACHINE.node,
    });
    expect(state.pages).toHaveLength(1);
    // Not once the person has gone on from the machine page.
    let chosen = openAddProjectFlow({ machines: [MACHINE, SECOND] });
    chosen = chooseAddProjectMachine(chosen, SECOND.node);
    chosen = applyAvailableMachines(chosen, [MACHINE]);
    expect(chosen.pages).toHaveLength(2);
    expect(chosen.machines).toEqual([MACHINE]);
  });

  it("restores page input and selection after Back", () => {
    let state = openAddProjectFlow({ machines: [MACHINE, SECOND] });
    state = setAddProjectPageInput(state, "rem");
    state = setAddProjectActiveIndex(state, 1);
    state = chooseAddProjectMachine(state, SECOND.node);
    state = openDirectorySearchPage(state, SECOND.node);
    state = backAddProjectPage(state) ?? state;
    state = backAddProjectPage(state) ?? state;
    expect(currentAddProjectPage(state)).toEqual({
      kind: "host",
      query: "rem",
      activeIndex: 1,
      error: null,
    });
  });

  it("wraps keyboard selection in both directions", () => {
    expect(moveAddProjectActiveIndex(2, 3, "next")).toBe(0);
    expect(moveAddProjectActiveIndex(0, 3, "previous")).toBe(2);
    expect(moveAddProjectSelection(0, [true, false, true], "next")).toBe(2);
    expect(moveAddProjectSelection(2, [true, false, true], "next")).toBe(0);
    expect(moveAddProjectSelection(1, [false, false], "next")).toBe(0);
  });

  it("restores a directory name after returning to and reselecting its parent", () => {
    let state = openAddProjectFlow({ machines: [MACHINE] });
    state = openNewDirectoryParentPage(state, MACHINE.node);
    state = openNewDirectoryNamePage(state, MACHINE.node, "~/dev");
    state = setNewDirectoryName(state, "command-center");
    state = backAddProjectPage(state) ?? state;
    state = openNewDirectoryNamePage(state, MACHINE.node, "~/dev");
    expect(currentAddProjectPage(state)).toMatchObject({
      kind: "new-directory-name",
      parentPath: "~/dev",
      name: "command-center",
    });
  });

  it("restores the GitHub destination query and active parent when reopening a repository", () => {
    let state = openAddProjectFlow({ machines: [MACHINE] });
    state = openGithubLocationPage(state, MACHINE.node, REPOSITORY);
    state = setAddProjectPageInput(state, "~/dev");
    state = setAddProjectActiveIndex(state, 2);
    state = backAddProjectPage(state) ?? state;
    state = openGithubLocationPage(state, MACHINE.node, REPOSITORY);
    expect(currentAddProjectPage(state)).toMatchObject({
      kind: "github-location",
      query: "~/dev",
      activeIndex: 2,
    });
  });

  it("sets a page's progress and error only while it is shown", () => {
    let state = openAddProjectFlow({ machines: [MACHINE] });
    state = openDirectorySearchPage(state, MACHINE.node);
    state = setAddProjectPageStatus(state, "directory-search", {
      isSubmitting: false,
      error: "Directory not found",
    });
    expect(currentAddProjectPage(state)).toMatchObject({
      error: "Directory not found",
    });
    // An answer for a page no longer shown changes nothing.
    state = backAddProjectPage(state) ?? state;
    const before = state;
    state = setAddProjectPageStatus(state, "directory-search", {
      error: "late",
    });
    expect(state).toEqual(before);
    // Typing clears the error.
    state = openDirectorySearchPage(state, MACHINE.node);
    state = setAddProjectPageStatus(state, "directory-search", { error: "x" });
    state = setAddProjectPageInput(state, "~/c");
    expect(currentAddProjectPage(state)).toMatchObject({
      query: "~/c",
      error: null,
    });
  });
});

describe("Add Project options", () => {
  it("hides every method when the machine cannot add projects", () => {
    const outdated = { ...MACHINE, canAddProject: false };
    expect(buildAddProjectMethods(outdated)).toEqual([]);
    expect(addProjectMethodEmptyText(outdated)).toBe(
      "Update the machine to use Add Project.",
    );
    expect(addProjectMethodEmptyText(MACHINE)).toBe("No matching options");
  });

  it("keeps machine-upgrade methods discoverable while hiding local-only Browse", () => {
    expect(
      buildAddProjectMethods({
        ...MACHINE,
        canBrowse: false,
        canCloneGithubRepositories: false,
        canSearchGithubRepositories: false,
        canCreateDirectory: false,
      }),
    ).toEqual([
      {
        id: "directory-search",
        label: "Search for directory",
        description: "Find a directory on Local",
      },
      {
        id: "github",
        label: "Clone from GitHub",
        description: "Update this machine to clone GitHub repositories",
        disabled: true,
      },
      {
        id: "new-directory",
        label: "New directory",
        description: "Update this machine to create directories",
        disabled: true,
      },
    ]);
  });

  it("offers Paseo's four methods in its order, Browse in the platform's file manager", () => {
    const methods = buildAddProjectMethods(
      { ...MACHINE, canSearchGithubRepositories: false },
      "File Explorer",
    );
    expect(methods.map((method) => method.id)).toEqual([
      "directory-search",
      "browse",
      "github",
      "new-directory",
    ]);
    expect(methods[1]?.description).toBe(
      "Choose or create a directory in File Explorer",
    );
    expect(methods[2]).toEqual({
      id: "github",
      label: "Clone from GitHub",
      description: "Enter a GitHub URL or owner/repo",
      disabled: false,
    });
    expect(methods[3]?.description).toBe("Create an empty directory on Local");
  });

  it("filters machines by name or key", () => {
    expect(filterAddProjectMachines([MACHINE, SECOND], " rem ")).toEqual([
      SECOND,
    ]);
    expect(filterAddProjectMachines([MACHINE, SECOND], "host-1")).toEqual([
      MACHINE,
    ]);
    expect(filterAddProjectMachines([MACHINE, SECOND], "")).toHaveLength(2);
  });

  it("offers manual URL and protocol-specific owner/repo clone choices", () => {
    expect(
      buildManualGithubRepositoryChoices("git@github.com:getpaseo/paseo.git"),
    ).toEqual([
      expect.objectContaining({
        id: "manual:git@github.com:getpaseo/paseo.git",
        nameWithOwner: "getpaseo/paseo",
        cloneUrl: "git@github.com:getpaseo/paseo.git",
      }),
    ]);
    expect(buildManualGithubRepositoryChoices("getpaseo/paseo")).toEqual([
      expect.objectContaining({
        cloneProtocol: "https",
        cloneUrl: "getpaseo/paseo",
      }),
      expect.objectContaining({
        cloneProtocol: "ssh",
        cloneUrl: "getpaseo/paseo",
      }),
    ]);
    expect(buildManualGithubRepositoryChoices("paseo")).toEqual([]);
  });

  it("clones owner/repo from the GitHub remote of its protocol", () => {
    const [https, ssh] = buildManualGithubRepositoryChoices("getpaseo/paseo");
    expect(https === undefined ? null : cloneRemote(https)).toBe(
      "https://github.com/getpaseo/paseo.git",
    );
    expect(ssh === undefined ? null : cloneRemote(ssh)).toBe(
      "git@github.com:getpaseo/paseo.git",
    );
    expect(cloneRemote(REPOSITORY)).toBe("git@github.com:getpaseo/paseo.git");
  });

  it("shows final clone paths while retaining parent paths as values", () => {
    expect(
      buildCloneLocationOptions({
        parents: ["~/dev", "~/workspace"],
        repositoryName: "paseo",
        existingPaths: ["~/workspace/paseo"],
      }),
    ).toEqual([
      {
        id: "~/dev",
        path: "~/dev",
        displayPath: "~/dev/paseo",
        secondaryText: "Parent directory: ~/dev",
        disabled: false,
      },
      {
        id: "~/workspace",
        path: "~/workspace",
        displayPath: "~/workspace/paseo",
        secondaryText: "Already exists",
        disabled: true,
      },
    ]);
  });

  it("shows equivalent absolute-home and tilde destinations only once", () => {
    expect(
      buildCloneLocationOptions({
        parents: ["/Users/moboudra/dev", "~/dev"],
        repositoryName: "dotfiles",
        existingPaths: [],
      }),
    ).toEqual([
      {
        id: "/Users/moboudra/dev",
        path: "/Users/moboudra/dev",
        displayPath: "/Users/moboudra/dev/dotfiles",
        secondaryText: "Parent directory: /Users/moboudra/dev",
        disabled: false,
      },
    ]);
  });

  it("suggests the projects' folders, then Paseo's usual places", () => {
    expect(
      buildSuggestedParentDirectories([
        "/home/me/code/shop",
        "/home/me/code/site",
        "/srv/app",
      ]),
    ).toEqual([
      "/home/me/code",
      "/srv",
      "~/dev",
      "~/Developer",
      "~/src",
      "~/projects",
      "~/workspace",
      "~",
    ]);
  });
});
