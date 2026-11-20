// Settings › Notifications (GOAT's "Notifications", Paseo's desktop
// section): the mode (off, notifications, sound, both), the browser's
// permission on this launch's address, a count in the tab's title, and
// a test.
//
// The browser grants notifications per address, and `vessel ui` opens at
// a new address every launch (ADR 0017), so the permission has to be
// given again after each launch; the section says so rather than
// pretend. Choosing a mode that shows notifications asks for it (a
// choice is a click, which Firefox requires); refused, the choice is not
// kept, as GOAT does.

import { useState } from "react";

import { useFleet } from "../../context";
import {
  alertsLabel,
  type Permission,
  permissionWords,
  playsSound,
  showsSystem,
} from "../../data/alerts";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import {
  Button,
  type Choice,
  Row,
  RowList,
  Select,
  Switch,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { Alerts, PreferenceChange } from "../../wire/bridge";
import { playNotice, unlockSound } from "../notifications/sound";
import { KeptNote } from "./kept-note";
import { SettingsPage, SettingsSection } from "./settings-page";

const MODES: readonly Choice<Alerts>[] = (
  ["off", "system", "sound", "system_and_sound"] as const
).map((value) => ({ value, label: alertsLabel(value) }));

function permissionNow(): Permission {
  return "Notification" in window ? Notification.permission : "unsupported";
}

async function askPermission(): Promise<Permission> {
  if (!("Notification" in window) || !window.isSecureContext) {
    return "unsupported";
  }
  try {
    return await Notification.requestPermission();
  } catch {
    return "unsupported";
  }
}

export function NotificationsSection() {
  const fleet = useFleet();
  const settings = useStore(prefs, (state) => state.values.notifications);
  const [permission, setPermission] = useState<Permission>(permissionNow);
  const [refused, setRefused] = useState<string | null>(null);
  const change = (next: PreferenceChange) => {
    void prefer(fleet, next).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const choose = async (alerts: Alerts) => {
    setRefused(null);
    if (playsSound(alerts)) {
      unlockSound();
    }
    if (showsSystem(alerts) && permission !== "granted") {
      const given = await askPermission();
      setPermission(given);
      if (given !== "granted") {
        setRefused(
          given === "unsupported"
            ? "Notifications are unavailable in this browser here. Sound only is still available."
            : "Allow notifications for this address in the browser's site settings, then choose this option again. Sound only is still available.",
        );
        return;
      }
    }
    change({ set: "alerts", value: alerts });
  };
  return (
    <SettingsPage lead="What this page tells you when a session needs you or finishes while you are elsewhere.">
      <KeptNote />
      <SettingsSection title="Sessions">
        <RowList label="Sessions">
          <Row
            title="Session notifications"
            detail={{
              description:
                refused ??
                "Alerts when a session finishes or needs your input or approval, unless you are looking at it.",
            }}
            control={
              <Select
                aria-label="Session notifications"
                size="sm"
                width="fit"
                align="end"
                value={settings.alerts}
                options={MODES}
                onChange={(alerts) => {
                  void choose(alerts);
                }}
              />
            }
          />
          <Row
            title="Count in the tab's title"
            detail={{
              description:
                "The tab's title shows how many sessions changed while you were away.",
            }}
            control={
              <Switch
                label="Count in the tab's title"
                checked={settings.title}
                onChange={(on) => {
                  change({ set: "title_count", value: on });
                }}
              />
            }
          />
        </RowList>
      </SettingsSection>
      <SettingsSection title="This launch">
        <RowList label="This launch">
          <Row
            title="Browser permission"
            detail={{
              description: `${permissionWords(permission)} Each launch of vessel ui opens at a new address, and the browser asks again for every address.`,
            }}
            control={
              permission === "default" ? (
                <Button
                  size="sm"
                  onClick={() => {
                    void askPermission().then(setPermission);
                  }}
                >
                  Allow
                </Button>
              ) : undefined
            }
          />
          <Row
            title="Test notification"
            detail={{ description: "Send one with these settings." }}
            control={
              <Button
                size="sm"
                disabled={settings.alerts === "off"}
                onClick={() => {
                  unlockSound();
                  if (
                    showsSystem(settings.alerts) &&
                    permission === "granted"
                  ) {
                    new Notification("Vessel notification test", {
                      body: "If you can see this, notifications work.",
                    });
                  }
                  if (playsSound(settings.alerts)) {
                    playNotice("needs-you");
                  }
                }}
              >
                Send
              </Button>
            }
          />
        </RowList>
      </SettingsSection>
    </SettingsPage>
  );
}
