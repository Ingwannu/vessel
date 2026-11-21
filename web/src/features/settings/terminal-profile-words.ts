// Settings › Terminals' words (ADR 0117): Paseo's
// `settings.host.terminalProfiles.*` (`app/src/i18n/resources/en.ts`
// 2955-2978), in English as the page's other Settings sections are.

export const PROFILE_WORDS = {
  title: "Terminal profiles",
  lead: "Terminals that start a program instead of the shell, listed under New tab.",
  unavailable: "Connect to this host to manage terminal profiles",
  update: "This machine's Vessel is too old for terminal profiles",
  add: "Add profile",
  editProfile: "Edit profile",
  addTitle: "Add terminal profile",
  editTitle: "Edit terminal profile",
  namePlaceholder: "Claude Code",
  commandPlaceholder: "claude",
  argsPlaceholder: "--dangerously-skip-permissions",
  nameLabel: "Name",
  commandLabel: "Command",
  argsLabel: "Arguments",
  nameRequired: "Name is required",
  commandRequired: "Command is required",
  argsHint: "Space-separated arguments passed to the command",
  saving: "Saving...",
  save: "Save",
  cancel: "Cancel",
  remove: "Remove",
  removeTitle: "Remove profile?",
  removeMessage: (name: string) => `Remove "${name}"?`,
  moveUp: "Move up",
  moveDown: "Move down",
  empty:
    "No profiles yet. Add one to launch terminals with a specific command.",
  unableToSave: "Unable to save",
} as const;
