#!/usr/bin/env python3
"""Shapes for Settings, scoped strictly to fleet.sh's disposable nodes.

Decision Log: the fixture does not redeem invitations or save pairing
links. It seeds inert device records with random, unredeemable hashes;
member reports are synthetic and loaded before the fake home starts.
This keeps a stable behind/conflict shape while the UI talks to real RPCs.
The second node has no devices, supplying the empty screen.
"""

import datetime
import json
import os
from pathlib import Path
import secrets
import sqlite3
import sys
import time

MEMBER = "00000000-0000-4000-8000-000000000021"
LONG = "A personal tablet with a very long name for checking settings rows in a narrow window and on a desktop"
KOREAN = "한글 기기 이름이 길어도 접근 해제 버튼과 겹치지 않아야 합니다"
REVOKED = "Retired phone (revoked fixture)"


def prepare(root: Path) -> None:
    home = root / "laptop"
    sources = root / "shared-settings"
    skills = sources / "skills" / "a-long-skill-name-for-checking-the-settings-file-list"
    skills.mkdir(parents=True, exist_ok=True)
    (sources / "AGENTS.md").write_text("# Shared instructions\nKeep changes reviewable.\n")
    (skills / "SKILL.md").write_text("---\nname: a-long-skill-name-for-checking-the-settings-file-list\ndescription: A disposable shared skill fixture\n---\n# A shared skill\nA disposable fixture.\n")
    (skills / "한글 안내.md").write_text("# 한글 안내\n공유 파일입니다.\n")
    home.mkdir(parents=True, exist_ok=True)
    # JSON double quoting is also valid for these TOML strings.
    (home / "fleet.toml").write_text(
        f"instructions = {json.dumps(str(sources / 'AGENTS.md'))}\n"
        f"skills = {json.dumps(str(sources / 'skills'))}\n"
        '[mcp.servers.docs]\ntype = "stdio"\ncommand = "docs-mcp"\n'
        'env = { TOKEN = { secret = "DOCS_TOKEN" }, MISSING = { secret = "SEARCH_KEY" } }\n'
    )
    # The app can request secret.list only; no fixture exposes this value.
    secret_file = home / "secrets.toml"
    secret_file.touch(mode=0o600)
    secret_file.write_text(f'DOCS_TOKEN = "{secrets.token_hex(32)}"\n')
    os.chmod(secret_file, 0o600)
    reports = home / "fleet"
    reports.mkdir(mode=0o700, exist_ok=True)
    (reports / "members.json").write_text(json.dumps({MEMBER: {
        "applied": 0, "conflicts": ["AGENTS.md", "skills/한글 안내.md"],
        "reported_at": (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(hours=2)).isoformat(),
    }}))


def seed(root: Path) -> None:
    now = time.time_ns()
    with sqlite3.connect(root / "laptop" / "vessel.db") as db:
        shapes = [(LONG, now - 180_000_000_000), (KOREAN, None)]
        shapes.extend((f"Test device {n}", now - n * 60_000_000_000) for n in range(3, 15))
        for n, (label, seen) in enumerate(shapes, 1):
            device = f"00000000-0000-4000-8000-{n:012d}"
            db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                       (device, label, secrets.token_bytes(32), '["read","operate","admin"]',
                        now - 172_800_000_000_000, seen, None))
        db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                   ("00000000-0000-4000-8000-000000000020", REVOKED,
                    secrets.token_bytes(32), '["read"]', now - 172_800_000_000_000, None, now))
        db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                   (MEMBER, "build-server", secrets.token_bytes(32), '["sync"]',
                    now - 172_800_000_000_000, now - 7_200_000_000_000, None))
    check(root)


def check(root: Path) -> None:
    with sqlite3.connect(root / "laptop" / "vessel.db") as db:
        rows = db.execute("SELECT label, last_seen, revoked_at FROM devices").fetchall()
    assert any(label == LONG and seen is not None for label, seen, _ in rows), "long active device missing"
    assert any(label == KOREAN and seen is None for label, seen, _ in rows), "unseen Korean device missing"
    assert any(label == REVOKED and revoked is not None for label, _, revoked in rows), "revoked device missing"
    assert len(rows) >= 16, "many-device shape missing"
    reports = json.loads((root / "laptop" / "fleet" / "members.json").read_text())
    assert reports[MEMBER]["applied"] == 0 and reports[MEMBER]["conflicts"], "behind/conflict member missing"
    assert (root / "laptop" / "fleet.toml").exists(), "home configuration missing"
    published = root / "laptop" / "fleet" / "manifest.json"
    if published.exists():
        manifest = json.loads(published.read_text())
        paths = list(manifest["files"])
        assert any(path.startswith("skills/") for path in paths), "published skill missing"
        assert any("한글" in path for path in paths), "published Korean skill path missing"


if __name__ == "__main__":
    action, location = sys.argv[1:]
    fixture_root = Path(location).resolve()
    {"prepare": prepare, "seed": seed, "check": check}[action](fixture_root)
