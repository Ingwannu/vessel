// Settings › General: how the composer sends (agent-orchestrator's
// "Chat send key", GOAT's and Paseo's "Send shortcut") and where the
// page's choices are kept (ADR 0028).

import { useFleet } from "../../context";
import { sendKeyLabel } from "../../data/compose-keys";
import { prefer, prefs } from "../../data/prefs";
import { currentPlatform } from "../../data/shortcuts/platform";
import { useStore } from "../../data/store";
import { type Choice, Row, RowList, Select } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { SendKey, TurnDelivery } from "../../wire/bridge";
import { KeptNote } from "./kept-note";
import { FactValue, SettingsPage, SettingsSection } from "./settings-page";

function sendChoices(): Choice<SendKey>[] {
  const primary = currentPlatform() === "mac" ? "⌘" : "Ctrl";
  return [
    { value: "enter", label: sendKeyLabel("enter", primary) },
    { value: "mod_enter", label: sendKeyLabel("mod_enter", primary) },
  ];
}

const DELIVERY_CHOICES: readonly Choice<TurnDelivery>[] = [
  { value: "queue", label: "Queue" },
  { value: "steer", label: "Steer" },
  { value: "interrupt", label: "Interrupt" },
];

/** What the delivery row says its choice does. */
function deliveryNote(delivery: TurnDelivery): string {
  switch (delivery) {
    case "queue":
      return "A message sent while the agent works waits, and goes when its turn ends.";
    case "steer":
      return "A message sent while the agent works joins its turn, where the agent can take one; it waits otherwise.";
    case "interrupt":
      return "A message sent while the agent works stops its turn and starts the next.";
  }
}

export function GeneralSection() {
  const fleet = useFleet();
  const send = useStore(prefs, (state) => state.values.composer.send);
  const delivery = useStore(prefs, (state) => state.values.composer.delivery);
  const kept = useStore(prefs, (state) => state.kept);
  return (
    <SettingsPage lead="How the composer sends, and where this page keeps what you choose.">
      <KeptNote />
      <SettingsSection title="Composer">
        <RowList label="Composer">
          <Row
            title="Send shortcut"
            detail={{
              description:
                send === "enter"
                  ? "Enter sends a prompt; Shift+Enter starts a new line."
                  : "Enter starts a new line; the shortcut sends.",
            }}
            control={
              <Select
                aria-label="Send shortcut"
                size="sm"
                width="fit"
                align="end"
                value={send}
                options={sendChoices()}
                onChange={(value) => {
                  void prefer(fleet, { set: "send", value }).then((outcome) => {
                    if (!outcome.ok) {
                      toast(outcome.problem.message, "error");
                    }
                  });
                }}
              />
            }
          />
          <Row
            title="Default send"
            detail={{ description: deliveryNote(delivery) }}
            control={
              <Select
                aria-label="Default send"
                size="sm"
                width="fit"
                align="end"
                value={delivery}
                options={DELIVERY_CHOICES}
                onChange={(value) => {
                  void prefer(fleet, { set: "delivery", value }).then(
                    (outcome) => {
                      if (!outcome.ok) {
                        toast(outcome.problem.message, "error");
                      }
                    },
                  );
                }}
              />
            }
          />
        </RowList>
      </SettingsSection>
      <SettingsSection title="Kept">
        <RowList label="Kept">
          <Row
            title="Preferences"
            detail={{
              description:
                "Kept by vessel ui on this machine, so every launch and every tab opens with them.",
            }}
            control={
              kept?.kind === "file" ? (
                <FactValue mono>{kept.path}</FactValue>
              ) : (
                <FactValue>This launch only</FactValue>
              )
            }
          />
        </RowList>
      </SettingsSection>
    </SettingsPage>
  );
}
