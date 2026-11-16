#!/usr/bin/env python3
"""Prepare isolated input for vessel-usage-fixture; never start a node or grant consent.

The marker confines the feature-enabled executable to a caller-created scratch
root. Source values are decoded by a test-only node backend, then admitted and
published by the production policy/cache service. No browser payload mocking.
"""
import argparse
import errno
import datetime as dt
import json
import os
import stat
import time
from pathlib import Path

MARKER = b"vessel-usage-fixture-v1\n"
RELEASE_WAIT_SECONDS = 2
RELEASE_RETRY_SECONDS = 0.01
CURSOR_STUB = b"vessel-usage-fixture-cursor-stub-v1\n"


def write(root, path, value):
    parent = root
    for component in path.parent.relative_to(root).parts:
        parent = parent / component
        if parent.is_symlink():
            raise SystemExit("refusing a fixture directory symlink")
        parent.mkdir(exist_ok=True, mode=0o700)
    raw = value if isinstance(value, bytes) else json.dumps(value, separators=(",", ":")).encode()
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(raw)


def report():
    now = dt.datetime.now(dt.timezone.utc)
    stamp = lambda hours: (now + dt.timedelta(hours=hours)).isoformat().replace("+00:00", "Z")
    return {
        "plan": "Synthetic source plan", "completeness": "partial",
        "windows": [
            {"id": "five-hour", "label": "Five-hour allowance", "short_label": {"kind": "text", "text": "5h"},
             "summary": "included", "used": 91, "remaining": 12, "resets_at": stamp(4),
             "runs_out_at": stamp(2), "shortfall": 17, "tone": "danger"},
            {"id": "weekly", "label": "Weekly allowance", "short_label": {"kind": "text", "text": "7d"},
             "summary": "included", "used": 70, "remaining": 28, "resets_at": stamp(48),
             "runs_out_at": None, "shortfall": None, "tone": "warning"}],
        "balances": [{"id": "credits", "label": "Synthetic credits", "used": 3, "remaining": 7, "limit": 10,
                      "unit": "credits", "resets_at": None, "tone": "ok"}],
        "details": [{"id": "fixture", "label": "Source", "value": "Recorded test values", "access": "read", "tone": None},
                    {"id": "private", "label": "Private detail", "value": "Operate-only synthetic fact", "access": "operate", "tone": None}],
    }


def marked(root):
    if (root / "usage-fixture.marker").read_bytes() != MARKER:
        raise SystemExit("refusing an unmarked root")
    for child in ("home", "data", "bin"):
        if not (root / child).resolve().is_relative_to(root):
            raise SystemExit("refusing a path outside the root")


def prepare(root, legacy):
    # Existing roots are refused: a typo must not overwrite a real sign-in.
    root.mkdir(parents=True, exist_ok=False, mode=0o700)
    for child in ("home", "data", "bin"):
        (root / child).mkdir(mode=0o700)
    write(root, root / "usage-fixture.marker", MARKER)
    write(root, root / "home/.config/cursor/auth.json", {"accessToken": "synthetic-cursor-A"})
    write(root, root / "source-report.json", report())
    if legacy:
        fixture = Path(__file__).resolve().parents[1] / "crates/vessel-testkit/tests/fixtures/limits/codex-usage.json"
        write(root, root / "legacy-body.json", fixture.read_bytes())
        for home, suffix in ((root / "home/.codex", "A"), (root / "home/codex-other", "B")):
            write(root, home / "auth.json", {"tokens": {"access_token": f"synthetic-codex-{suffix}"}})
        write(root, root / "data/profiles.toml", b'[profiles.fixture-other]\nharness = "codex"\nlabel = "Other synthetic account"\n[profiles.fixture-other.env]\nCODEX_HOME = "~/codex-other"\n')


def hold(root):
    if not hasattr(os, "mkfifo"):
        raise SystemExit("the fixture gate requires Unix FIFOs")
    stub = root / "home/.config/Cursor/User/globalStorage/state.vscdb"
    if stub.exists() and (stub.stat().st_size != len(CURSOR_STUB) or stub.read_bytes() != CURSOR_STUB):
        raise SystemExit("refusing to overwrite a non-fixture Cursor database")
    write(root, stub, CURSOR_STUB)
    os.mkfifo(root / "source-gate", mode=0o600)


def release(root):
    gate = root / "source-gate"
    before = gate.lstat()
    if not stat.S_ISFIFO(before.st_mode):
        raise SystemExit("refusing a non-FIFO fixture gate")
    deadline = time.monotonic() + RELEASE_WAIT_SECONDS
    while True:
        try:
            descriptor = os.open(gate, os.O_WRONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
            break
        except OSError as error:
            if error.errno != errno.ENXIO or time.monotonic() >= deadline:
                raise SystemExit("no recorded source read is waiting at the gate") from None
            time.sleep(RELEASE_RETRY_SECONDS)
    try:
        current = os.fstat(descriptor)
        if (current.st_dev, current.st_ino) != (before.st_dev, before.st_ino):
            raise SystemExit("the fixture gate changed")
        os.write(descriptor, b"r")
    finally:
        os.close(descriptor)
    after = gate.lstat()
    if (after.st_dev, after.st_ino) != (before.st_dev, before.st_ino):
        raise SystemExit("the fixture gate changed")
    gate.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("action", choices=("prepare", "malformed", "restore", "switch-login", "hold", "release"))
    parser.add_argument("--legacy", action="store_true", help="also prepare two distinct saved Codex profiles")
    args = parser.parse_args()
    root = args.root.absolute()
    if args.action == "prepare":
        prepare(root, args.legacy)
    else:
        root = root.resolve(strict=True)
        marked(root)
        if args.action == "hold":
            hold(root)
        elif args.action == "release":
            release(root)
        elif args.action == "malformed":
            write(root, root / "source-report.json", b'{"malformed":')
        elif args.action == "restore":
            write(root, root / "source-report.json", report())
        else:
            write(root, root / "home/.config/cursor/auth.json", {"accessToken": "synthetic-cursor-B"})


if __name__ == "__main__":
    main()
