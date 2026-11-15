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
import tomllib
import time

MEMBER = "00000000-0000-4000-8000-000000000021"
CURRENT = "00000000-0000-4000-8000-000000000022"
UNREPORTED = "00000000-0000-4000-8000-000000000023"
LONG_MEMBER = "A member machine with a long name for checking the shared settings status on a narrow screen"
KOREAN_MEMBER = "아직 동기화 결과를 보고하지 않은 한글 머신"
PRIVATE_LITERAL = "do-not-render-this-fixture-setting"
LONG = "A personal tablet with a very long name for checking settings rows in a narrow window and on a desktop"
KOREAN = "한글 기기 이름이 길어도 접근 해제 버튼과 겹치지 않아야 합니다"
REVOKED = "Retired phone (revoked fixture)"


def prepare(root: Path) -> None:
    home = root / "laptop"
    sources = root / "shared-settings"
    skills = sources / "skills" / "a-long-skill-name-for-checking-the-settings-file-list"
    skills.mkdir(parents=True, exist_ok=True)
    (sources / "AGENTS.md").write_text("# Shared instructions\n" + "Keep changes reviewable.\n" * 128)
    (skills / "SKILL.md").write_text("---\nname: a-long-skill-name-for-checking-the-settings-file-list\ndescription: A disposable shared skill fixture\n---\n# A shared skill\nA disposable fixture.\n")
    (skills / "한글 안내.md").write_text("# 한글 안내\n공유 파일입니다.\n")
    script = skills / "check.sh"
    script.write_text("#!/bin/sh\nexit 0\n")
    script.chmod(0o700)
    home.mkdir(parents=True, exist_ok=True)
    # JSON double quoting is also valid for these TOML strings.
    (home / "fleet.toml").write_text(
        f"instructions = {json.dumps(str(sources / 'AGENTS.md'))}\n"
        f"skills = {json.dumps(str(sources / 'skills'))}\n"
        '[mcp.servers.docs]\ntype = "stdio"\ncommand = "docs-mcp"\n'
        'env = { TOKEN = { secret = "DOCS_TOKEN" }, MISSING = { secret = "SEARCH_KEY" }, LITERAL = "' + PRIVATE_LITERAL + '" }\n'
        '[mcp.servers.search]\ntype = "http"\nurl = "https://example.invalid/mcp"\n'
        'headers = { Authorization = { secret = "SEARCH_KEY" }, "X-Mode" = "' + PRIVATE_LITERAL + '" }\n'
    )
    # The app can request secret.list only; no fixture exposes this value.
    secret_file = home / "secrets.toml"
    secret_file.touch(mode=0o600)
    secret_file.write_text(f'DOCS_TOKEN = "{secrets.token_hex(32)}"\nLOCAL_NOTE = "{secrets.token_hex(32)}"\n')
    os.chmod(secret_file, 0o600)
    reports = home / "fleet"
    reports.mkdir(mode=0o700, exist_ok=True)
    (reports / "members.json").write_text(json.dumps({MEMBER: {
        "applied": 0, "conflicts": ["AGENTS.md", "skills/한글 안내.md"],
        "reported_at": (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(hours=2)).isoformat(),
    }, CURRENT: {
        "applied": 1, "conflicts": [],
        "reported_at": (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(minutes=1)).isoformat(),
    }}))


def seed(root: Path) -> None:
    now = time.time_ns()
    with sqlite3.connect(root / "laptop" / "vessel.db") as db:
        shapes = [(LONG, now - 180_000_000_000), (KOREAN, None)]
        shapes.extend((f"Test device {n}", now - n * 60_000_000_000) for n in range(3, 15))
        for n, (label, seen) in enumerate(shapes, 1):
            device = f"00000000-0000-4000-8000-{n:012d}"
            scopes = '["read"]' if label == KOREAN else '["read","operate"]' if n == 3 else '["read","operate","admin"]'
            db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                       (device, label, secrets.token_bytes(32), scopes,
                        now - 172_800_000_000_000, seen, None))
        db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                   ("00000000-0000-4000-8000-000000000020", REVOKED,
                    secrets.token_bytes(32), '["read"]', now - 172_800_000_000_000, None, now))
        db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                   (MEMBER, "build-server", secrets.token_bytes(32), '["sync"]',
                    now - 172_800_000_000_000, now - 7_200_000_000_000, None))
        for device, label, seen in [(CURRENT, LONG_MEMBER, now - 60_000_000_000), (UNREPORTED, KOREAN_MEMBER, None)]:
            db.execute("INSERT INTO devices VALUES (?, ?, ?, ?, ?, ?, ?)",
                       (device, label, secrets.token_bytes(32), '["sync"]', now - 172_800_000_000_000, seen, None))
    manifest_file = root / "laptop" / "fleet" / "manifest.json"
    for _ in range(100):
        if manifest_file.exists():
            break
        time.sleep(0.05)
    assert manifest_file.exists(), "published fixture did not become ready"
    check(root)


def empty(root: Path) -> None:
    """Instructions-only home: successful publication with no server names."""
    source = root / "empty-settings" / "instructions.md"
    source.parent.mkdir(exist_ok=True)
    source.write_text("# Disposable instructions-only home.\n")
    path = root / "build-server" / "fleet.toml"
    path.write_text(f"instructions = {json.dumps(str(source))}\n")
    assert set(tomllib.loads(path.read_text())) == {"instructions"}, "instructions-only shape missing"


def files_empty(root: Path) -> None:
    """Server-only home: successful publication with no shared files."""
    skills = root / "empty-settings" / "skills"
    skills.mkdir(parents=True, exist_ok=True)
    path = root / "build-server" / "fleet.toml"
    path.write_text(f"skills = {json.dumps(str(skills))}\n" +
                    '[mcp.servers.fixture]\ntype = "stdio"\ncommand = "fixture-noop-mcp"\n')
    assert set(tomllib.loads(path.read_text())) == {"skills", "mcp"}, "server-only shape missing"
    assert not any(skills.iterdir()), "empty skills directory has files"


def standalone(root: Path) -> None:
    """Return the second node to the default, no-home shape."""
    (root / "build-server" / "fleet.toml").unlink(missing_ok=True)


def row_access(root: Path) -> None:
    """Upgrade only named inert fixtures in an already-created fake fleet."""
    with sqlite3.connect(root / "laptop" / "vessel.db") as db:
        for device, label, scopes in [
            ("00000000-0000-4000-8000-000000000002", KOREAN, '["read"]'),
            ("00000000-0000-4000-8000-000000000003", "Test device 3", '["read","operate"]'),
        ]:
            changed = db.execute("UPDATE devices SET scopes=? WHERE id=? AND label=?", (scopes, device, label)).rowcount
            assert changed == 1, "named inert permission fixture missing"
    check(root)


def check(root: Path) -> None:
    with sqlite3.connect(root / "laptop" / "vessel.db") as db:
        rows = db.execute("SELECT label, last_seen, revoked_at FROM devices").fetchall()
        access = dict(db.execute("SELECT label, scopes FROM devices"))
    assert json.loads(access[KOREAN]) == ["read"], "read-only row missing"
    assert json.loads(access["Test device 3"]) == ["read", "operate"], "session-access row missing"
    assert any(label == LONG and seen is not None for label, seen, _ in rows), "long active device missing"
    assert any(label == KOREAN and seen is None for label, seen, _ in rows), "unseen Korean device missing"
    assert any(label == REVOKED and revoked is not None for label, _, revoked in rows), "revoked device missing"
    assert len(rows) >= 18, "many-device shape missing"
    assert any(label == LONG_MEMBER for label, _, _ in rows), "long member missing"
    assert any(label == KOREAN_MEMBER for label, _, _ in rows), "Korean member missing"
    values = tomllib.loads((root / "laptop" / "secrets.toml").read_text())
    assert "DOCS_TOKEN" in values and "LOCAL_NOTE" in values and "SEARCH_KEY" not in values, "secret-name presence shapes missing"
    assert (root / "shared-settings" / "AGENTS.md").stat().st_size > 1024, "readable size shape missing"
    reports = json.loads((root / "laptop" / "fleet" / "members.json").read_text())
    assert reports[MEMBER]["applied"] == 0 and reports[MEMBER]["conflicts"], "behind/conflict member missing"
    assert CURRENT in reports and reports[CURRENT]["applied"] > 0, "current member missing"
    assert UNREPORTED not in reports, "unreported member must have no invented report"
    assert (root / "laptop" / "fleet.toml").exists(), "home configuration missing"
    published = root / "laptop" / "fleet" / "manifest.json"
    if published.exists():
        manifest = json.loads(published.read_text())
        paths = list(manifest["files"])
        assert any(path.startswith("skills/") for path in paths), "published skill missing"
        assert any("한글" in path for path in paths), "published Korean skill path missing"
        assert any(file.get("executable") for file in manifest["files"].values()), "executable skill missing"
        assert set(manifest["mcp"]) == {"docs", "search"}, "server-name shapes missing"
        assert reports[CURRENT]["applied"] == manifest["generation"], "current member version differs"


if __name__ == "__main__":
    action, location = sys.argv[1:]
    fixture_root = Path(location).resolve()
    {"prepare": prepare, "seed": seed, "check": check, "empty": empty, "files-empty": files_empty, "standalone": standalone, "row-access": row_access}[action](fixture_root)
