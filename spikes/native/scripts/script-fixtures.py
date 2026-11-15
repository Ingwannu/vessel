#!/usr/bin/env python3
"""Seed inert scripts for native lifecycle captures on a private fake fleet.

The fixture creates a project, but leaves approval and process starts to the app.
It refuses a non-loopback book, live-preview ports and non-fake adapters.
"""
from pathlib import Path
import json
import os
import socket
import subprocess
import sys
import tomllib
from urllib.parse import urlparse

FORBIDDEN_PORTS = {7077, 18220, 18221}


def main() -> None:
    root, target = (Path(arg).resolve() for arg in sys.argv[1:])
    assert root != Path.home(), "an isolated fleet root is required"
    assert (root / "nodes").is_file(), "fleet marker is missing"
    book = tomllib.loads((root / "config/vessel/nodes.toml").read_text())
    assert set(book["nodes"]) == {"laptop", "build-server"}, "unexpected book entries"
    for entry in book["nodes"].values():
        routes = entry.get("direct", []) + ([entry["url"]] if "url" in entry else [])
        assert routes, "a local route is required"
        for route in routes:
            address = urlparse(route)
            assert address.hostname in {"127.0.0.1", "localhost", "::1"}, "loopback routes only"
            assert address.port not in FORBIDDEN_PORTS, "live-preview route refused"
    adapter = (root / "laptop-bin/claude-agent-acp").read_text()
    assert str(target / "fake-acp") in adapter, "a real adapter was refused"
    project = root / "work/script-lifecycle-fixture"
    project.mkdir(parents=True, exist_ok=False)
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    assert port not in FORBIDDEN_PORTS, "reserved preview port"
    config = f'''[scripts.check]
command = "printf 'fixture complete\\n'"

[scripts.fail-with-exit-code]
command = "exit 7"

[services.preview-with-a-long-name-for-phone-layout]
command = "/usr/bin/python3 -u -m http.server {port} --bind 127.0.0.1"
'''
    (project / "vessel.toml").write_text(config)
    (project / "index.html").write_text("<!doctype html><title>Private script preview</title><h1>Private script preview</h1>")
    env = {"HOME": str(root), "VESSEL_CONFIG_DIR": str(root / "config"), "PATH": "/usr/bin:/bin"}
    added = subprocess.run([str(target / "vessel"), "--node", "laptop", "project", "add", str(project)],
                           env=env, capture_output=True, text=True, timeout=30, check=False)
    assert added.returncode == 0, "fixture project registration failed"
    (root / "script-fixture.json").write_text(json.dumps({"project": project.name, "port": port, "approved": False}) + "\n")
    print("script fixture prepared; approve and run from the app")


if __name__ == "__main__":
    main()
