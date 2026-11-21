// Settings › Terminals' dialogs (ADR 0117): Paseo's terminal profile edit
// modal (`screens/settings/terminal-profile-edit-modal.tsx`) and its remove
// confirmation (`host-page.tsx` 1503-1527).
//
// [Decision Log]
// - Reference: Name, Command and Arguments; name and command are required
//   after trimming, each error shown under its field after a save and
//   cleared as the person types; Enter moves Name → Command → Arguments,
//   and Enter in Arguments saves; Save reads "Saving..." while it waits; a
//   refused save keeps the dialog open with the reason under the fields.
//   Arguments split on whitespace, without quoting (`parseArgsString`).
// - Choice: the kit's dialog, fields and buttons. Stated difference: the
//   kit's medium dialog is 520 wide where Paseo's modal is 480.

import { useRef, useState } from "react";

import { parseArgs } from "../../data/terminal-profiles";
import { Button, Dialog, Field, TextField } from "../../ui/kit";
import type { TerminalProfile } from "../../wire/proto";
import { PROFILE_WORDS } from "./terminal-profile-words";

/** What the dialog saves: the fields, trimmed and split. */
export interface ProfileDraft {
  name: string;
  command: string;
  args: string[];
}

export function ProfileEditDialog({
  profile,
  onSave,
  onClose,
}: {
  /** The profile edited; `null` to add one. */
  profile: TerminalProfile | null;
  /** Resolves with the refusal's words, or `null` once kept. */
  onSave: (draft: ProfileDraft) => Promise<string | null>;
  onClose: () => void;
}) {
  const [name, setName] = useState(profile?.name ?? "");
  const [command, setCommand] = useState(profile?.command ?? "");
  const [args, setArgs] = useState(profile?.args.join(" ") ?? "");
  const [nameError, setNameError] = useState<string | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const commandField = useRef<HTMLInputElement>(null);
  const argsField = useRef<HTMLInputElement>(null);

  const save = async () => {
    const missingName = name.trim() === "";
    const missingCommand = command.trim() === "";
    setNameError(missingName ? PROFILE_WORDS.nameRequired : null);
    setCommandError(missingCommand ? PROFILE_WORDS.commandRequired : null);
    if (missingName || missingCommand || saving) {
      return;
    }
    setSaving(true);
    setRefused(null);
    const problem = await onSave({
      name: name.trim(),
      command: command.trim(),
      args: parseArgs(args),
    });
    setSaving(false);
    if (problem === null) {
      onClose();
    } else {
      setRefused(problem);
    }
  };

  return (
    <Dialog
      title={
        profile === null ? PROFILE_WORDS.addTitle : PROFILE_WORDS.editTitle
      }
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {PROFILE_WORDS.cancel}
          </Button>
          <Button onClick={() => void save()} disabled={saving}>
            {saving ? PROFILE_WORDS.saving : PROFILE_WORDS.save}
          </Button>
        </>
      }
    >
      <div className="terminal-profile-form">
        <Field label={PROFILE_WORDS.nameLabel} error={nameError}>
          <TextField
            value={name}
            autoFocus
            placeholder={PROFILE_WORDS.namePlaceholder}
            onChange={(next) => {
              setName(next);
              setNameError(null);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") commandField.current?.focus();
            }}
          />
        </Field>
        <Field label={PROFILE_WORDS.commandLabel} error={commandError}>
          <TextField
            value={command}
            mono
            spellCheck={false}
            inputRef={commandField}
            placeholder={PROFILE_WORDS.commandPlaceholder}
            onChange={(next) => {
              setCommand(next);
              setCommandError(null);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") argsField.current?.focus();
            }}
          />
        </Field>
        <Field label={PROFILE_WORDS.argsLabel} hint={PROFILE_WORDS.argsHint}>
          <TextField
            value={args}
            mono
            spellCheck={false}
            inputRef={argsField}
            placeholder={PROFILE_WORDS.argsPlaceholder}
            onChange={setArgs}
            onKeyDown={(event) => {
              if (event.key === "Enter") void save();
            }}
          />
        </Field>
        {refused === null ? null : (
          <p className="terminal-profile-refused" role="alert">
            {refused}
          </p>
        )}
      </div>
    </Dialog>
  );
}

export function ProfileRemoveDialog({
  profile,
  onKeep,
  onRemove,
}: {
  profile: TerminalProfile;
  onKeep: () => void;
  onRemove: () => void;
}) {
  return (
    <Dialog
      title={PROFILE_WORDS.removeTitle}
      onClose={onKeep}
      footer={
        <>
          <Button variant="ghost" onClick={onKeep}>
            {PROFILE_WORDS.cancel}
          </Button>
          <Button variant="danger-solid" onClick={onRemove}>
            {PROFILE_WORDS.remove}
          </Button>
        </>
      }
    >
      <p className="dialog-copy">{PROFILE_WORDS.removeMessage(profile.name)}</p>
    </Dialog>
  );
}
