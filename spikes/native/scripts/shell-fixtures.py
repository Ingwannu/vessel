#!/usr/bin/env python3
"""Real-shape shell fixtures, only on fleet.sh's disposable fake nodes.

Decision Log: create and mark sessions through the real CLI and node;
read the durable projection to assert the long/many/pinned/archived
shapes. No actual harness, user configuration, or pairing is involved.
"""

from pathlib import Path
import sqlite3
import subprocess
import sys
import tomllib
from urllib.parse import urlparse

ENGLISH = "Review the shell's command palette with long session titles and paths while preserving the reference's keyboard order"
KOREAN = "긴 한국어 세션 제목과 경로가 좁은 화면에서 키보드 탐색이나 다른 행의 글자와 겹치지 않는지 확인해 주세요. 데스크톱과 휴대폰 크기에서 같은 순서를 유지해야 합니다"


def endpoints(node: dict) -> list[str]:
    """Every address the book would dial for `node`.

    fleet.sh pairs each node as a person does, so an entry is a paired
    one (`key` with its `direct` and `relay` routes, no `url`); a book
    written by an older fleet.sh holds `url`. An `ssh` entry, or one with
    no address at all, has none here and is refused by the caller.
    """
    if "url" in node:
        return [node["url"]]
    return [*node.get("direct", []), *node.get("relay", [])]


def check_fleet(root: Path) -> None:
    book = tomllib.loads((root / "config/vessel/nodes.toml").read_text())
    for node in book["nodes"].values():
        addresses = endpoints(node)
        if not addresses:
            raise ValueError("Shell fixtures require disposable loopback nodes")
        for address in addresses:
            parsed = urlparse(address)
            if parsed.hostname != "127.0.0.1" or parsed.port in {7077, 18220, 18221}:
                raise ValueError("Shell fixtures require disposable loopback nodes")
    adapter = (root / "bin/claude-agent-acp").read_text()
    if "fake-acp" not in adapter or "FAKE_ACP_OPTIONS=real" not in adapter:
        raise ValueError("Shell fixtures require the real-shape fake adapter")


def seed(root: Path, target: Path) -> None:
    check_fleet(root)
    env = {
        "PATH": f"{root / 'bin'}:/usr/bin:/bin",
        "HOME": str(root),
        "VESSEL_CONFIG_DIR": str(root / "config"),
    }

    def cli(*args: str) -> str:
        result = subprocess.run([str(target / "vessel"), "--node", "laptop", *args],
                                env=env, capture_output=True, text=True, timeout=30)
        if result.returncode:
            # No command output is reflected: a future CLI could include
            # a sensitive value in a failure, even in a fake environment.
            raise RuntimeError(f"Shell fixture command failed: {args[0]}")
        return result.stdout.strip()

    for index in range(8):
        folder = root / "work" / "a-long-project-name-for-checking-palette-paths" / f"shell-{index}"
        folder.mkdir(parents=True, exist_ok=True)
        session = cli("run", "claude-code", "--structured", "--cwd", str(folder), "--detach").splitlines()[-1]
        title = ENGLISH if index % 2 == 0 else KOREAN
        cli("send", session, f"{title} ({index})")
        if index < 2:
            cli("session", "pin", session)
        if index >= 6:
            cli("session", "archive", session)
    check(root)


def check(root: Path) -> None:
    with sqlite3.connect(f"file:{root / 'laptop/vessel.db'}?mode=ro", uri=True) as db:
        rows = db.execute("SELECT title, pinned, archived_at FROM sessions").fetchall()
    assert len(rows) >= 8, "many-session shell shape missing"
    assert sum(bool(title) and len(title) >= 70 for title, _, _ in rows) >= 8, "long-title shell shape missing"
    assert any(title and "한국어" in title for title, _, _ in rows), "Korean shell shape missing"
    assert sum(bool(pinned) for _, pinned, _ in rows) >= 2, "pinned shell shape missing"
    assert sum(archived is not None for _, _, archived in rows) >= 2, "archive shell shape missing"


if __name__ == "__main__":
    action, location, *rest = sys.argv[1:]
    fixture_root = Path(location).resolve()
    if action == "seed":
        seed(fixture_root, Path(rest[0]).resolve())
    elif action == "check":
        check(fixture_root)
    else:
        raise ValueError("Expected seed or check")
