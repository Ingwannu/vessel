// Settings › Terminals: a machine's terminal profiles (ADR 0117), Paseo's
// host Terminals page (`screens/settings/host-page.tsx` 1311-1665).
//
// [Decision Log]
// - Reference: one row per profile, in the list's order, with its mark,
//   its name, and its command and arguments as a hint; Move up, Move down
//   (off at the ends), Edit and Remove (asks first; its trash in the
//   danger colour, Paseo's `destructiveColorMapping`). Add sits at the
//   section's end. Every change saves the whole list. A host not connected
//   shows one line saying to connect; an empty list says how to add one.
// - Choice: the kit's rows and icon buttons, the machine picker beside Add
//   as every machine section has it. While a save waits the row buttons
//   wait too, so two changes never race. A refused move or removal says
//   Paseo's "Unable to save" with the node's reason.
// - Not here: Paseo's "Terminal agents" card above the list (ADR 0089's
//   hooks switch; P6). A node too old for profiles says so (Paseo shows
//   nothing for a missing feature).

import "./terminals-section.css";

import { useState } from "react";

import { useFleetState } from "../../context";
import { machineFor } from "../../data/machines";
import {
  commandLine,
  newProfileId,
  profilesServed,
} from "../../data/terminal-profiles";
import { Alert, IconButton, Row, RowList, Skeleton } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { TerminalProfile } from "../../wire/proto";
import { MachinePicker } from "../machines/machine-picker";
import { ProfileGlyph } from "../workspace-tabs/profile-rows";
import {
  type ProfileDraft,
  ProfileEditDialog,
  ProfileRemoveDialog,
} from "./profile-dialogs";
import { SettingsPage, SettingsSection } from "./settings-page";
import { PROFILE_WORDS } from "./terminal-profile-words";
import { useTerminalProfileSettings } from "./use-terminal-profiles";

export function TerminalsSection({
  machine,
}: {
  machine: NodeKey | undefined;
}) {
  const book = useFleetState((state) => state.book);
  const selected = useFleetState((state) => state.selected);
  const node = machineFor(book ?? [], machine, selected);
  if (node === null) {
    return (
      <SettingsPage lead={PROFILE_WORDS.lead}>
        {book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )}
      </SettingsPage>
    );
  }
  return <TerminalsOf key={node.key} machine={node.key} />;
}

type Editing = { kind: "add" } | { kind: "edit"; profile: TerminalProfile };

function moved(list: readonly TerminalProfile[], index: number, by: -1 | 1) {
  const next = [...list];
  const other = index + by;
  const here = next[index];
  const there = next[other];
  if (here === undefined || there === undefined) return list;
  next[index] = there;
  next[other] = here;
  return next;
}

/** One machine's profiles; keyed by the machine. */
function TerminalsOf({ machine }: { machine: NodeKey }) {
  const status = useFleetState((state) => state.statuses[machine]);
  const connected = status?.state === "connected";
  const supported = connected && profilesServed(status.protocol);
  const hold = useTerminalProfileSettings(machine, supported);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [removing, setRemoving] = useState<TerminalProfile | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const picker = <MachinePicker section="terminals" machine={machine} />;
  const list = hold.listed?.ok === true ? hold.listed.value : null;

  const keep = async (
    next: readonly TerminalProfile[],
  ): Promise<string | null> => {
    const result = await hold.save(next);
    return result.ok ? null : result.problem.message;
  };
  const change = (next: readonly TerminalProfile[]) => {
    setProblem(null);
    void keep(next).then((refused) => {
      if (refused !== null) setProblem(refused);
    });
  };
  const saveDraft = (draft: ProfileDraft) => {
    if (list === null || editing === null)
      return Promise.resolve(PROFILE_WORDS.unableToSave);
    if (editing.kind === "add") {
      const id = newProfileId(Date.now(), Math.random());
      return keep([...list, { id, ...draft }]);
    }
    const edited = editing.profile;
    return keep(
      list.map((row) => (row.id === edited.id ? { ...row, ...draft } : row)),
    );
  };

  const add = (
    <IconButton
      icon="plus"
      label={PROFILE_WORDS.addTitle}
      variant="ghost"
      size="sm"
      disabled={list === null || hold.saving}
      onClick={() => {
        setEditing({ kind: "add" });
      }}
    />
  );
  const action = (
    <div className="terminal-profiles-actions">
      {picker}
      {supported ? add : null}
    </div>
  );

  let body;
  if (!connected) {
    body = <p className="settings-empty">{PROFILE_WORDS.unavailable}</p>;
  } else if (!supported) {
    body = <Alert size="md" variant="warning" title={PROFILE_WORDS.update} />;
  } else if (hold.listed === null) {
    body = <Skeleton />;
  } else if (!hold.listed.ok) {
    body = (
      <Alert size="md" variant="error" title={hold.listed.problem.message} />
    );
  } else if (hold.listed.value.length === 0) {
    body = <p className="settings-empty">{PROFILE_WORDS.empty}</p>;
  } else {
    const rows = hold.listed.value;
    body = (
      <RowList label={PROFILE_WORDS.title}>
        {rows.map((profile, index) => (
          <Row
            key={profile.id}
            mark={{
              node: (
                <span className="row-icon">
                  <ProfileGlyph profile={profile} size={16} />
                </span>
              ),
            }}
            title={profile.name}
            detail={{
              description: (
                <code className="terminal-profile-command">
                  {commandLine(profile)}
                </code>
              ),
            }}
            control={
              <div className="terminal-profile-buttons">
                <IconButton
                  icon="arrow-up"
                  label={PROFILE_WORDS.moveUp}
                  variant="ghost"
                  size="sm"
                  disabled={index === 0 || hold.saving}
                  onClick={() => {
                    change(moved(rows, index, -1));
                  }}
                />
                <IconButton
                  icon="arrow-down"
                  label={PROFILE_WORDS.moveDown}
                  variant="ghost"
                  size="sm"
                  disabled={index === rows.length - 1 || hold.saving}
                  onClick={() => {
                    change(moved(rows, index, 1));
                  }}
                />
                <IconButton
                  icon="edit"
                  label={PROFILE_WORDS.editProfile}
                  variant="ghost"
                  size="sm"
                  disabled={hold.saving}
                  onClick={() => {
                    setEditing({ kind: "edit", profile });
                  }}
                />
                <IconButton
                  icon="trash"
                  label={PROFILE_WORDS.remove}
                  variant="danger"
                  size="sm"
                  disabled={hold.saving}
                  onClick={() => {
                    setRemoving(profile);
                  }}
                />
              </div>
            }
          />
        ))}
      </RowList>
    );
  }

  return (
    <SettingsPage lead={PROFILE_WORDS.lead}>
      <SettingsSection title={PROFILE_WORDS.title} action={action}>
        {problem === null ? null : (
          <Alert
            size="sm"
            variant="error"
            title={PROFILE_WORDS.unableToSave}
            description={problem}
          />
        )}
        {body}
      </SettingsSection>
      {editing === null ? null : (
        <ProfileEditDialog
          profile={editing.kind === "edit" ? editing.profile : null}
          onSave={saveDraft}
          onClose={() => {
            setEditing(null);
          }}
        />
      )}
      {removing === null || list === null ? null : (
        <ProfileRemoveDialog
          profile={removing}
          onKeep={() => {
            setRemoving(null);
          }}
          onRemove={() => {
            const gone = removing.id;
            setRemoving(null);
            change(list.filter((row) => row.id !== gone));
          }}
        />
      )}
    </SettingsPage>
  );
}
