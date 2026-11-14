#!/usr/bin/env python3
"""Check N3 wire shapes through the fake fleet's CLI, without agent prompts."""
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid


def prepare(root: Path, _target: Path) -> None:
    # Existing Settings sources make laptop the home. Only disposable
    # harness homes are materialized; never the operator's real profiles.
    for name in ["laptop", "build-server"]:
        for harness in [".claude", ".codex"]:
            (root / f"{name}-home" / harness).mkdir(parents=True, exist_ok=True)


def check(root: Path, target: Path) -> None:
    env = {"PATH": os.environ["PATH"], "HOME": str(root),
           "VESSEL_CONFIG_DIR": str(root / "config"), "TZ": "UTC"}
    readonly_env = {**env, "VESSEL_CONFIG_DIR": str(root / "readonly-config")}
    def call(*args: str, node: str = "laptop", data: str = "", readonly: bool = False, config: str | None = None) -> subprocess.CompletedProcess[str]:
        selected = readonly_env if readonly else env
        if config is not None:
            selected = {**env, "VESSEL_CONFIG_DIR": str(root / config)}
        return subprocess.run([str(target / "vessel"), "--node", node, *args],
                              env=selected, input=data, capture_output=True, text=True,
                              timeout=30, check=False)
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
    def success(*args: str, node: str = "laptop", data: str = "", readonly: bool = False, config: str | None = None) -> str:
        result = call(*args, node=node, data=data, readonly=readonly, config=config)
        # Invitation output is deliberately never included in a failure.
        assert result.returncode == 0, f"wire fixture refused {args[0]} {args[1]}"
        return result.stdout

    def pipe_invitation(sender: list[str], receiver: list[str], receive_env: dict[str, str]) -> None:
        # Never materialize a pairing link in a string, argument or log.
        # Popen owns the producer PID until reaped, including error cleanup.
        producer = subprocess.Popen([str(target / "vessel"), *sender], env=env,
                                    stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            assert producer.stdout is not None
            paired = subprocess.run([str(target / "vessel"), *receiver], env=receive_env,
                                    stdin=producer.stdout, capture_output=True, timeout=30,
                                    check=False)
            producer.stdout.close()
            sent = producer.wait(timeout=30)
            assert sent == 0 and paired.returncode == 0, "invitation pipe refused"
        finally:
            if producer.stdout is not None:
                producer.stdout.close()
            if producer.poll() is None:
                producer.kill()
                producer.wait()

    def join(label: str) -> None:
        pipe_invitation(["--node", "laptop", "fleet", "invite", "--label", label],
                        ["--node", "build-server", "fleet", "join", "--yes", "--grant", "none"], env)

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
    # Separate books avoid duplicate identities/shadowing in the app book.
    pipe_invitation(["--node", "laptop", "node", "pair", "--read-only", "--label", "Read-only Sync fixture"],
                    ["nodes", "add", "laptop", "--pair", "--yes"], readonly_env)
    readonly = success("fleet", "status", readonly=True)
    assert readonly.splitlines()[0] == "role  home", "read-only role missing"
    assert label not in readonly and "build-server" not in readonly, "read-only leaked member identities"
    assert call("fleet", "sync", readonly=True).returncode != 0, "read-only sync was permitted"
    assert call("fleet", "leave", readonly=True).returncode != 0, "read-only leave was permitted"
    for name in ["laptop", "build-server"]:
        own = success("node", "devices", "self", node=name).splitlines()
        assert len(own) == 2 and own[1] == "scopes  read, operate, admin", "person grant shape missing"
        assert own[0].startswith("device  "), "paired self identity label missing"
        uuid.UUID(own[0].removeprefix("device  "))
    assert success("node", "devices", "self", config="operator-config") == (
        "device  none\nscopes  read, operate, admin, sync, relay\n"), "owner self grant missing"
    read_self = success("node", "devices", "self", readonly=True).splitlines()
    assert len(read_self) == 2 and read_self[1] == "scopes  read", "read-only self grant missing"
    assert read_self[0].startswith("device  "), "reader self identity label missing"
    uuid.UUID(read_self[0].removeprefix("device  "))
    sync_env = {**env, "VESSEL_CONFIG_DIR": str(root / "sync-only-config")}
    pipe_invitation(["--node", "laptop", "fleet", "invite", "--label", "Sync-only self fixture"],
                    ["nodes", "add", "laptop", "--pair", "--yes"], sync_env)
    sync_self = success("node", "devices", "self", config="sync-only-config").splitlines()
    assert len(sync_self) == 2 and sync_self[1] == "scopes  sync", "self without Read refused or widened"
    assert sync_self[0].startswith("device  "), "no-Read self identity label missing"
    uuid.UUID(sync_self[0].removeprefix("device  "))
    assert call("status", config="sync-only-config").returncode != 0, "self granted Read"
    assert call("node", "devices", config="sync-only-config").returncode != 0, "self granted Admin"
    (root / "wire-gap-shapes.txt").write_text(
        "timezone, ten runs, CLI zone, impossible expression: checked\n"
        "fleet home/member/standalone, join/leave/sync, applied reports, read-only refusal: checked\n"
        "self: paired person, local owner, reader and no-Read caller; no widened grant: checked\n")


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
    except ValueError:
        print("wire-gaps check failed: self identity shape invalid", file=sys.stderr)
        sys.exit(1)
    except OSError:
        print(f"wire-gaps check failed: {action}: fixture I/O failed", file=sys.stderr)
        sys.exit(1)
