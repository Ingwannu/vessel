#!/usr/bin/env python3
# ADDED by Vessel (Team-WICKED), 2026-11-19 (spikes/release/patches/README.md,
# "gpui-pre-linux", change 16): the test server of
# `src/linux/system_notifications.rs`, not part of the published crate.
"""A stand-in org.freedesktop.Notifications for ADR 0114's Linux proof.

It runs on a private session bus (never the desktop's). Each Notify is
recorded as one JSON line in LOG: id, app name, replaced id, summary, body
and actions; each CloseNotification as {"closed": id}. A test asks it to act
through its own interface (org.vessel.FakeNotifications.Invoke), which emits
ActionInvoked and then NotificationClosed, as a notification server does
when a body is clicked.

Usage: fake_notifyd.py LOG
"""
import json
import sys

import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

SPEC = "org.freedesktop.Notifications"
CONTROL = "org.vessel.FakeNotifications"


class Notifications(dbus.service.Object):
    def __init__(self, bus, log):
        super().__init__(bus, "/org/freedesktop/Notifications")
        self.log = log
        self.last = 0

    def record(self, line):
        with open(self.log, "a", encoding="utf-8") as out:
            out.write(json.dumps(line) + "\n")

    @dbus.service.method(SPEC, in_signature="susssasa{sv}i", out_signature="u")
    def Notify(self, app, replaces, icon, summary, body, actions, hints, timeout):
        if int(replaces) > 0:
            number = int(replaces)
        else:
            self.last += 1
            number = self.last
        self.record(
            {
                "id": number,
                "app": str(app),
                "replaces": int(replaces),
                "summary": str(summary),
                "body": str(body),
                "actions": [str(action) for action in actions],
            }
        )
        return dbus.UInt32(number)

    @dbus.service.method(SPEC, in_signature="", out_signature="as")
    def GetCapabilities(self):
        return ["body", "actions"]

    @dbus.service.method(SPEC, in_signature="", out_signature="ssss")
    def GetServerInformation(self):
        return ("vessel-fake-notifyd", "vessel", "1", "1.2")

    @dbus.service.method(SPEC, in_signature="u", out_signature="")
    def CloseNotification(self, number):
        self.record({"closed": int(number)})
        self.NotificationClosed(number, dbus.UInt32(3))

    @dbus.service.signal(SPEC, signature="uu")
    def NotificationClosed(self, number, reason):
        pass

    @dbus.service.signal(SPEC, signature="us")
    def ActionInvoked(self, number, key):
        pass

    @dbus.service.method(CONTROL, in_signature="us", out_signature="")
    def Invoke(self, number, key):
        self.ActionInvoked(dbus.UInt32(number), key)
        self.NotificationClosed(dbus.UInt32(number), dbus.UInt32(2))


def main():
    DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    _name = dbus.service.BusName(SPEC, bus)
    _server = Notifications(bus, sys.argv[1])
    print("FAKE_NOTIFYD_READY", flush=True)
    GLib.MainLoop().run()


if __name__ == "__main__":
    main()
