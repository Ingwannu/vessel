#!/usr/bin/env python3
"""Record exact disposable node identities; stop only a still-matching PID."""
import json
import os
from pathlib import Path
import signal
import select
import sys


def identity(pid: int) -> tuple[Path, list[str], str]:
    proc = Path(f"/proc/{pid}")
    assert proc.stat().st_uid == os.getuid(), "node UID changed"
    exe = (proc / "exe").resolve(strict=True)
    args = (proc / "cmdline").read_bytes().decode().rstrip("\0").split("\0")
    start = (proc / "stat").read_text().rsplit(")", 1)[1].split()[19]
    return exe, args, start


def record(root: Path, pid: int, binary: Path, data: Path) -> None:
    exe, args, start = identity(pid)
    assert exe == binary.resolve(), "node executable differs"
    assert "--data-dir" in args and args[args.index("--data-dir") + 1] == str(data), "node data directory differs"
    path = root / "processes.json"
    entries = json.loads(path.read_text()) if path.exists() else []
    entries.append({"pid": pid, "exe": str(exe), "data": str(data), "start": start})
    path.write_text(json.dumps(entries, indent=2) + "\n")


def stop(root: Path) -> None:
    path = root / "processes.json"
    if not path.exists():
        assert not (root / "pids").exists(), "no recorded identity; inspect old fleet PIDs manually"
        return
    for entry in json.loads(path.read_text()):
        pid = entry["pid"]
        try:
            # Pin before inspection, so PID reuse cannot redirect a signal.
            descriptor = os.pidfd_open(pid)
        except ProcessLookupError:
            continue
        try:
            exited = select.poll()
            exited.register(descriptor, select.POLLIN)
            if exited.poll(0):
                continue
            exe, args, start = identity(pid)
            assert start == entry["start"] and str(exe) == entry["exe"], "node identity changed"
            assert "--data-dir" in args and args[args.index("--data-dir") + 1] == entry["data"], "node data directory changed"
            signal.pidfd_send_signal(descriptor, signal.SIGTERM)
            assert exited.poll(30000), "node did not stop; recorded identities retained"
        except ProcessLookupError:
            pass
        finally:
            os.close(descriptor)
    path.unlink()
    (root / "pids").unlink(missing_ok=True)


if __name__ == "__main__":
    try:
        action, location, *rest = sys.argv[1:]
        root = Path(location).resolve()
        if action == "record":
            pid, binary, data = rest
            record(root, int(pid), Path(binary), Path(data))
        elif action == "stop":
            stop(root)
        else:
            raise ValueError("unknown operation")
    except (AssertionError, OSError, ValueError, KeyError, IndexError) as error:
        detail = str(error) if isinstance(error, AssertionError) else "process inspection failed"
        print(f"fleet process check failed: {detail}", file=sys.stderr)
        sys.exit(1)
