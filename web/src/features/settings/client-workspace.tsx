import "./client-workspace.css";

import { useState } from "react";

import { useFleet } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { WORKSPACE_SPLIT_QUERY } from "../../data/workspace/tab-layout";
import { Row, RowList, Select, TextField, type Choice } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type {
  OpenLocationDestination,
  OpenLocationSource,
  PreferenceChange,
  PullRequestLocation,
} from "../../wire/bridge";
import { useMedia } from "../shell/use-media";
import { KeptNote } from "./kept-note";
import { SettingsPage, SettingsSection } from "./settings-page";

const MAX_SCROLLBACK = 1_000_000;

function usePreferenceEdit() {
  const fleet = useFleet();
  return (change: PreferenceChange) => {
    void prefer(fleet, change).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
}

export function TerminalSection() {
  const scrollback = useStore(
    prefs,
    (state) => state.values.terminal.scrollback,
  );
  const [draft, setDraft] = useState<string | null>(null);
  const edit = usePreferenceEdit();
  const save = () => {
    setDraft(null);
    if (draft === null || draft === "") {
      return;
    }
    const lines = Number(draft);
    const next = Math.min(lines, MAX_SCROLLBACK);
    if (Number.isFinite(lines) && next !== scrollback) {
      edit({ set: "terminal_scrollback", value: next });
    }
  };
  return (
    <SettingsPage>
      <KeptNote />
      <SettingsSection title="Terminal">
        <RowList label="Terminal">
          <Row
            title="Terminal scrollback"
            detail={{
              description: "Lines kept in the built-in terminal buffer",
            }}
            control={
              <div className="terminal-scrollback-field">
                <TextField
                  aria-label="Terminal scrollback lines"
                  size="sm"
                  inputMode="numeric"
                  selectOnFocus
                  value={draft ?? String(scrollback)}
                  onChange={(text) => {
                    setDraft(text.replace(/\D/gu, ""));
                  }}
                  onBlur={save}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.currentTarget.blur();
                    }
                  }}
                />
              </div>
            }
          />
        </RowList>
      </SettingsSection>
    </SettingsPage>
  );
}

export function FilesSection() {
  return (
    <SettingsPage>
      <SettingsSection title="Files">
        <p className="file-associations-empty">
          File panels appear here when a plugin adds one
        </p>
      </SettingsSection>
    </SettingsPage>
  );
}

const LOCATIONS: readonly Choice<OpenLocationDestination>[] = [
  { value: "main", label: "Main panel" },
  { value: "side", label: "On the side" },
];

const PULL_LOCATIONS: readonly Choice<PullRequestLocation>[] = [
  ...LOCATIONS,
  { value: "explorer", label: "Explorer sidebar" },
];

const SOURCES: readonly {
  source: OpenLocationSource;
  label: string;
}[] = [
  {
    source: "explorer_files",
    label: "Opening a file from the Files tab",
  },
  {
    source: "diffs",
    label: "Opening Changes or Diff from the workspace or a chat",
  },
  { source: "diff_files", label: "Clicking a file in a diff" },
];

export function OpenLocationSection() {
  const wide = useMedia(WORKSPACE_SPLIT_QUERY);
  const locations = useStore(prefs, (state) => state.values.open_location);
  const edit = usePreferenceEdit();
  if (!wide) {
    return null;
  }
  return (
    <SettingsSection title="Open location">
      <RowList label="Open location">
        {SOURCES.map(({ source, label }) => (
          <Row
            key={source}
            title={<span className="open-location-label">{label}</span>}
            control={
              <Select
                aria-label={label}
                size="sm"
                width="fit"
                align="end"
                value={locations[source]}
                options={LOCATIONS}
                onChange={(destination) => {
                  edit({
                    set: "open_location",
                    value: { source, destination },
                  });
                }}
              />
            }
          />
        ))}
        <Row
          title={
            <span className="open-location-label">
              Opening a pull request from New tab
            </span>
          }
          control={
            <Select
              aria-label="Opening a pull request from New tab"
              size="sm"
              width="fit"
              align="end"
              value={locations.pull_requests}
              options={PULL_LOCATIONS}
              onChange={(value) => {
                edit({ set: "pull_request_location", value });
              }}
            />
          }
        />
      </RowList>
    </SettingsSection>
  );
}
