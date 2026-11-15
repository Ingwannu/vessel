#!/usr/bin/env python3
"""Check N3 wire shapes through the fake fleet's CLI, without agent prompts."""
import os
from pathlib import Path
import subprocess
import sys
import time


def prepare(root: Path, _target: Path) -> None:
    # Shared typed inputs; no app preferences, account files or credentials.
    fixture_dir = root / "launch-profile-fixtures"
    fixture_dir.mkdir(mode=0o700, exist_ok=True)
    source = Path(__file__).resolve().parents[3] / "crates/vessel-testkit/fixtures"
    for stem in ["replace", "options"]:
        (fixture_dir / f"{stem}.json").write_text(
            (source / f"launch-profiles-{stem}.json").read_text(encoding="utf-8"), encoding="utf-8")
    (fixture_dir / "apply.json").write_text(
        '{"session":"131d469e-0b08-4596-bb5a-93b5c0ce9cc0","options":[]}', encoding="utf-8")
    # Existing Settings sources make laptop the home. Only disposable
    # harness homes are materialized; never the operator's real profiles.
    for name in ["laptop", "build-server"]:
        for harness in [".claude", ".codex"]:
            (root / f"{name}-home" / harness).mkdir(parents=True, exist_ok=True)


def check(root: Path, target: Path) -> None:
    env = {"PATH": os.environ["PATH"], "HOME": str(root),
           "VESSEL_CONFIG_DIR": str(root / "config"), "TZ": "UTC"}
    readonly_env = {**env, "VESSEL_CONFIG_DIR": str(root / "readonly-config")}
    def call(*args: str, node: str = "laptop", data: str = "", readonly: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run([str(target / "vessel"), "--node", node, *args],
                              env=readonly_env if readonly else env, input=data, capture_output=True, text=True,
                              timeout=30, check=False)
    # Pending is not an empty host list or permission proof. After assignment,
    # integration replaces this negative fixture with numbered positive calls.
    pending = root / "launch-profile-fixtures"
    for args, method in [
        (["list"], "launch_profile.list"),
        (["replace", str(pending / "replace.json")], "launch_profile.replace"),
        (["options", str(pending / "options.json")], "launch_profile.options"),
        (["applicable", "131d469e-0b08-4596-bb5a-93b5c0ce9cc0"], "launch_profile.applicable"),
        (["apply", str(pending / "apply.json")], "session.apply_options"),
    ]:
        result = call("launch-profiles", *args)
        assert result.returncode != 0 and not result.stdout, "pending launch call appeared successful"
        assert method in result.stderr and "not assigned" in result.stderr, "pending launch call refusal missing"
    status = call("status")
    assert status.returncode == 0, "fake node status refused"
    assert "timezone  Asia/Seoul\n" in status.stdout, "node time zone shape missing"
    preview = call("schedule", "preview", "--cron", "15 9 * * 1-5",
                   "--tz", "UTC", "--after", "2026-01-05T09:14:30Z", "--count", "10")
    assert preview.returncode == 0, "schedule preview refused"
    assert preview.stdout.splitlines() == [
        f"2026-01-{day:02d}T09:15:00+00:00" for day in [5, 6, 7, 8, 9, 12, 13, 14, 15, 16]
    ], "many-run preview shape missing"
    viewer_zone = call("schedule", "preview", "--cron", "0 9 * * *",
                        "--after", "2026-01-05T00:00:00Z", "--count", "1")
    assert viewer_zone.returncode == 0, "viewer-zone preview refused"
    assert viewer_zone.stdout == "2026-01-05T09:00:00+00:00\n", "default CLI zone not used"
    bad = call("schedule", "preview", "--cron", "0 0 30 2 *", "--tz", "UTC")
    assert bad.returncode != 0, "impossible schedule did not refuse"
    def success(*args: str, node: str = "laptop", data: str = "", readonly: bool = False) -> str:
        result = call(*args, node=node, data=data, readonly=readonly)
        # Invitation output is deliberately never included in a failure.
        assert result.returncode == 0, f"wire fixture refused {args[0]} {args[1]}"
        return result.stdout

    def join(label: str) -> None:
        link = success("fleet", "invite", "--label", label)
        success("fleet", "join", "--yes", "--grant", "none", node="build-server", data=link)

    assert success("fleet", "status").splitlines()[0] == "role  home", "home role missing"
    assert success("fleet", "status", node="build-server").splitlines()[0] == "role  no fleet", "standalone role missing"
    assert call("fleet", "sync", node="build-server").returncode != 0, "standalone sync did not refuse"
    join("wire-controls-retired")
    success("fleet", "leave", node="build-server")
    assert success("fleet", "status", node="build-server").splitlines()[0] == "role  no fleet", "leave did not persist"
    devices = success("node", "devices").splitlines()
    retired = [line.split()[0] for line in devices if "wire-controls-retired" in line]
    assert len(retired) == 1, "retired fixture identity missing"
    success("node", "devices", "revoke", retired[0])
    label = "Connected build server with a long name for applied-state rows"
    join(label)
    success("fleet", "sync")
    success("fleet", "sync", node="build-server")
    deadline = time.monotonic() + 30
    while True:
        state = success("fleet", "status", node="build-server")
        members = success("fleet", "status")
        applied = (root / "build-server-home/.codex/AGENTS.md").is_file()
        if applied and "last applied  1\n" in state and f"{label} generation 1" in members:
            break
        assert time.monotonic() < deadline, "real member apply/report did not arrive"
        time.sleep(0.1)  # Fixture deadline, never production coordination.
    readonly_link = success("node", "pair", "--read-only", "--label", "Read-only Sync fixture")
    # A separate book avoids duplicate identities/shadowing in the normal
    # fleet. Select readonly-config when capturing restricted UI states.
    paired = subprocess.run([str(target / "vessel"), "nodes", "add", "laptop", "--pair", "--yes"],
                            env=readonly_env, input=readonly_link, text=True, capture_output=True,
                            timeout=30, check=False)
    assert paired.returncode == 0, "read-only fixture pairing refused"
    readonly = success("fleet", "status", readonly=True)
    assert readonly.splitlines()[0] == "role  home", "read-only role missing"
    assert label not in readonly and "build-server" not in readonly, "read-only leaked member identities"
    assert call("fleet", "sync", readonly=True).returncode != 0, "read-only sync was permitted"
    assert call("fleet", "leave", readonly=True).returncode != 0, "read-only leave was permitted"
    (root / "wire-gap-shapes.txt").write_text(
        "named launch calls: five pending refusals checked; no positive/permission claim\n"
        "timezone, ten runs, CLI zone, impossible expression: checked\n"
        "fleet home/member/standalone, join/leave/sync, applied reports, read-only refusal: checked\n")


if __name__ == "__main__":
    action, location, binary_dir = sys.argv[1:]
    try:
        {"prepare": prepare, "check": check}[action](Path(location).resolve(), Path(binary_dir).resolve())
    except AssertionError as error:
        print(f"wire-gaps check failed: {error}", file=sys.stderr)
        sys.exit(1)
    except subprocess.TimeoutExpired:
        print(f"wire-gaps check failed: {action}: fixture command timed out", file=sys.stderr)
        sys.exit(1)
    except OSError:
        print(f"wire-gaps check failed: {action}: fixture I/O failed", file=sys.stderr)
        sys.exit(1)
