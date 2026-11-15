"""Tests of patch-feed.py (ADR 0099), through its command line.

    python3 -I -B -m unittest discover -s spikes/release/tools -p 'test_*.py'

Each test builds a stand-in executable (random bytes around the marker),
runs the tool as the release does, and checks what it wrote and what it
said. Every run's output is searched for the address and its token, which
must never appear.
"""

import os
import re
import secrets
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL = HERE / "patch-feed.py"
CONFIG = HERE.parent / "updater" / "src" / "config.rs"

TOKEN = "tok" + secrets.token_hex(16)
ADDRESS = f"https://feed.example.test/{TOKEN}/"


def rust_constants():
    """FEED_SLOT_LEN, FEED_SLOT_HEADER and FEED_SLOT_FILL as config.rs has them."""
    text = CONFIG.read_text()
    length = re.search(r"(?:pub(?:\(crate\))? )?const FEED_SLOT_LEN: usize = (\d+);", text)
    header = re.search(r'const FEED_SLOT_HEADER: &\[u8\] = b"([^"]*)";', text)
    fill = re.search(r"const FEED_SLOT_FILL: u8 = b'(.)';", text)
    assert length and header and fill, "config.rs no longer declares the slot as this test reads it"
    return int(length.group(1)), header.group(1).encode("ascii"), fill.group(1).encode("ascii")


SLOT_LEN, HEADER, FILL = rust_constants()
MARKER = HEADER + FILL * (SLOT_LEN - len(HEADER))


class Run:
    def __init__(self, completed):
        self.code = completed.returncode
        self.out = completed.stdout
        self.err = completed.stderr


class PatchFeed(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp(prefix="patch-feed-test-"))
        self.address = self.dir / "address"
        self.address.write_text(ADDRESS + "\n")

    def tearDown(self):
        for path in sorted(self.dir.rglob("*"), reverse=True):
            path.unlink() if not path.is_dir() else path.rmdir()
        self.dir.rmdir()

    def tool(self, *arguments):
        completed = subprocess.run(
            [sys.executable, "-I", str(TOOL), *map(str, arguments)],
            capture_output=True,
            text=True,
            check=False,
        )
        run = Run(completed)
        for stream in (run.out, run.err):
            self.assertNotIn(TOKEN, stream)
            self.assertNotIn("feed.example.test", stream)
        return run

    def executable(self, name="app", markers=1, before=4096, after=8192, mode=0o755):
        parts = [os.urandom(before)]
        for _ in range(markers):
            parts += [MARKER, os.urandom(333)]
        parts.append(os.urandom(after))
        path = self.dir / name
        path.write_bytes(b"".join(parts))
        path.chmod(mode)
        return path, before

    def leftovers(self):
        return [path.name for path in self.dir.iterdir() if path.name.startswith(".patch-feed-")]

    def test_the_constants_are_config_rs_constants(self):
        tool = TOOL.read_text()
        self.assertIn(f"SLOT_LEN = {SLOT_LEN}\n", tool)
        self.assertIn(f'HEADER = b"{HEADER.decode()}"\n', tool)
        self.assertIn(f'FILL = b"{FILL.decode()}"\n', tool)

    def test_write_puts_the_address_in_the_slot_and_nothing_else_moves(self):
        source, at = self.executable()
        original = source.read_bytes()
        target = self.dir / "app.patched"
        run = self.tool("write", "--address-file", self.address, source, target)
        self.assertEqual(run.code, 0, run.err)
        self.assertIn(f"offset {at:#x}", run.out)
        written = target.read_bytes()
        self.assertEqual(len(written), len(original))
        self.assertEqual(written[:at], original[:at])
        self.assertEqual(written[at + SLOT_LEN :], original[at + SLOT_LEN :])
        slot = written[at : at + SLOT_LEN]
        self.assertEqual(slot, ADDRESS.encode() + b"\0" * (SLOT_LEN - len(ADDRESS)))
        self.assertNotIn(MARKER, written)
        # The source is never changed; the copy keeps its mode.
        self.assertEqual(source.read_bytes(), original)
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o755)
        self.assertEqual(self.leftovers(), [])

    def test_a_windows_executable_keeps_its_mode_too(self):
        source, _ = self.executable("app.exe", mode=0o644)
        target = self.dir / "Vessel.exe"
        self.assertEqual(self.tool("write", "--address-file", self.address, source, target).code, 0)
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o644)

    def test_no_slot_or_two_slots_are_refused_and_nothing_is_written(self):
        for markers in (0, 2):
            with self.subTest(markers=markers):
                source, _ = self.executable(f"app{markers}", markers=markers)
                target = self.dir / f"out{markers}"
                run = self.tool("write", "--address-file", self.address, source, target)
                self.assertEqual(run.code, 1)
                self.assertIn(f"occurs {markers} times", run.err)
                self.assertFalse(target.exists())
                self.assertEqual(self.leftovers(), [])

    def test_a_header_without_its_fill_is_not_a_slot(self):
        path = self.dir / "app"
        path.write_bytes(os.urandom(100) + HEADER + b"x" * SLOT_LEN)
        run = self.tool("check", path)
        self.assertEqual(run.code, 1)
        self.assertIn("occurs 0 times (its header 1 times)", run.err)

    def test_an_existing_target_is_refused(self):
        source, _ = self.executable()
        target = self.dir / "there"
        target.write_bytes(b"keep me")
        run = self.tool("write", "--address-file", self.address, source, target)
        self.assertEqual(run.code, 1)
        self.assertEqual(target.read_bytes(), b"keep me")

    def test_the_longest_address_fits_and_one_more_byte_does_not(self):
        source, at = self.executable()
        longest = "https://feed.example.test/" + "a" * (SLOT_LEN - 1 - len("https://feed.example.test/"))
        self.address.write_text(longest + "\n")
        target = self.dir / "long"
        self.assertEqual(self.tool("write", "--address-file", self.address, source, target).code, 0)
        self.assertEqual(target.read_bytes()[at : at + SLOT_LEN], longest.encode() + b"\0")
        self.address.write_text(longest + "a\n")
        run = self.tool("write", "--address-file", self.address, source, self.dir / "longer")
        self.assertEqual(run.code, 1)
        self.assertIn(f"at most {SLOT_LEN - 1}", run.err)

    def test_addresses_the_app_would_refuse_are_refused(self):
        source, _ = self.executable()
        bad = [
            "",
            "http://feed.example.test/x/",
            f"https://feed.example.test/{TOKEN}/?a=b",
            f"https://feed.example.test/{TOKEN}/#a",
            f"https://user:{TOKEN}@feed.example.test/",
            f"https://feed.example.test/{TOKEN} /",
            "ftp://feed.example.test/",
            "https:///nohost/",
        ]
        for number, address in enumerate(bad):
            with self.subTest(address=number):
                self.address.write_text(address + "\n")
                target = self.dir / f"bad{number}"
                run = self.tool("write", "--address-file", self.address, source, target)
                self.assertEqual(run.code, 1)
                self.assertFalse(target.exists())

    def test_a_test_feed_on_this_machine_is_allowed(self):
        source, _ = self.executable()
        for number, address in enumerate(["http://127.0.0.1:8123/proof/", "http://localhost:1/", "http://[::1]:2/"]):
            with self.subTest(address=address):
                self.address.write_text(address + "\n")
                run = self.tool("write", "--address-file", self.address, source, self.dir / f"local{number}")
                self.assertEqual(run.code, 0, run.err)

    def test_a_crlf_line_and_more_lines_after_it_are_fine(self):
        source, at = self.executable()
        self.address.write_bytes(ADDRESS.encode() + b"\r\nsecond line\n")
        target = self.dir / "crlf"
        self.assertEqual(self.tool("write", "--address-file", self.address, source, target).code, 0)
        self.assertTrue(target.read_bytes()[at:].startswith(ADDRESS.encode() + b"\0"))

    def test_check_and_verify(self):
        source, _ = self.executable()
        self.assertEqual(self.tool("check", source).code, 0)
        self.assertEqual(self.tool("verify", "--address-file", self.address, source).code, 1)
        target = self.dir / "written"
        self.assertEqual(self.tool("write", "--address-file", self.address, source, target).code, 0)
        self.assertEqual(self.tool("check", target).code, 1)
        self.assertEqual(self.tool("verify", "--address-file", self.address, target).code, 0)
        # Another address is not this one.
        other = self.dir / "other"
        other.write_text("https://feed.example.test/another/\n")
        self.assertEqual(self.tool("verify", "--address-file", other, target).code, 1)
        # A byte after the padding changed: no longer a written slot.
        data = bytearray(target.read_bytes())
        at = data.find(ADDRESS.encode())
        data[at + SLOT_LEN - 1] = ord("x")
        target.write_bytes(bytes(data))
        self.assertEqual(self.tool("verify", "--address-file", self.address, target).code, 1)

    def test_writing_twice_is_refused(self):
        source, _ = self.executable()
        once = self.dir / "once"
        self.assertEqual(self.tool("write", "--address-file", self.address, source, once).code, 0)
        run = self.tool("write", "--address-file", self.address, once, self.dir / "twice")
        self.assertEqual(run.code, 1)
        self.assertIn("occurs 0 times", run.err)

    def test_a_missing_address_file_is_refused_without_a_traceback(self):
        source, _ = self.executable()
        run = self.tool("write", "--address-file", self.dir / "none", source, self.dir / "out")
        self.assertEqual(run.code, 1)
        self.assertNotIn("Traceback", run.err)


if __name__ == "__main__":
    unittest.main()
