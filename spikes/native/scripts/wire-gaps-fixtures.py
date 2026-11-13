#!/usr/bin/env python3
"""Check N3 wire shapes through the fake fleet's CLI, without agent prompts."""
import os
from pathlib import Path
import subprocess
import sys


def check(root: Path, target: Path) -> None:
    env = {"PATH": os.environ["PATH"], "HOME": str(root),
           "VESSEL_CONFIG_DIR": str(root / "config"), "TZ": "UTC"}
    def call(*args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([str(target / "vessel"), "--node", "laptop", *args],
                              env=env, capture_output=True, text=True, timeout=30, check=False)
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
    (root / "wire-gap-shapes.txt").write_text("timezone, ten runs, CLI zone, impossible expression: checked\n")


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
