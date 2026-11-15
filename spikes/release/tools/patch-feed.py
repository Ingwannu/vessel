"""Write the update feed's address into a built Vessel app (ADR 0099).

    python3 -I patch-feed.py check FILE...
    python3 -I patch-feed.py write --address-file ADDRESS IN OUT
    python3 -I patch-feed.py verify --address-file ADDRESS FILE...

The app is compiled with a slot of SLOT_LEN bytes holding a marker
(updater/src/config.rs) on a machine that holds no secret; the machine that
signs the release writes the feed's address into exactly those bytes, here,
before it packages, sums and signs.

  check   each FILE holds the unwritten slot exactly once (the compile
          step's output, before anything is written)
  write   IN must hold the unwritten slot exactly once; OUT (which must not
          exist) gets IN's bytes and mode with the slot replaced by the
          address and NUL padding to SLOT_LEN, written to a temporary file
          beside OUT and renamed into place. The written file is read back
          and checked: the same size, every byte outside the slot equal to
          IN's, the slot the address and padding, no marker left. IN is
          never changed.
  verify  each FILE holds the written address exactly once, followed by NUL
          padding to SLOT_LEN, and no marker (the copies that ship, after
          packaging)

ADDRESS is a file whose first line is the address. The address carries the
feed's token: nothing here prints it, puts it in an error, or writes it
anywhere but OUT. Messages name offsets, lengths and SHA-256 sums of the
executables only.

Exit status: 0 done, 1 a check failed (OUT not created), 2 usage.

Why bytes and not the executable's sections: the slot is a plain static
array, its bytes in the file are its bytes in memory on both Linux (ELF
.rodata) and Windows (PE .rdata), with no relocation inside it, so finding
the marker in the file is finding the slot. Counting the whole marker, not
just its header, is what makes "exactly once" mean the slot itself.

Run with `python3 -I` (no current-directory imports). Standard library only.
"""

import argparse
import hashlib
import ipaddress
import os
import sys
import tempfile
import urllib.parse

# The same three as updater/src/config.rs (FEED_SLOT_LEN, FEED_SLOT_HEADER,
# FEED_SLOT_FILL); test_patch_feed.py reads them from there and fails when
# they differ.
SLOT_LEN = 512
HEADER = b"VESSEL-UPDATE-FEED-SLOT/ADR-0099/UNPATCHED/"
FILL = b"#"
MARKER = HEADER + FILL * (SLOT_LEN - len(HEADER))


class Refused(Exception):
    """A check failed; the message never holds the address."""


def say(line):
    print(line, flush=True)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def occurrences(data, needle):
    """Every offset of `needle` in `data`, overlapping ones included."""
    found = []
    start = 0
    while True:
        at = data.find(needle, start)
        if at < 0:
            return found
        found.append(at)
        start = at + 1


def read_address(path):
    """The address in the first line of `path`, checked as the app checks a
    feed's address (updater/src/feed.rs: https://, or http:// to this
    machine for a test feed; no query, fragment or credentials)."""
    try:
        with open(path, "rb") as source:
            first = source.readline()
    except OSError as error:
        raise Refused(f"cannot read the address file: {error.strerror}") from None
    text = first.strip()
    if not text:
        raise Refused("the address file's first line is empty")
    if len(text) >= SLOT_LEN:
        raise Refused(f"the address is {len(text)} bytes; the slot takes at most {SLOT_LEN - 1}")
    if any(byte <= 0x20 or byte >= 0x7F for byte in text):
        raise Refused("the address holds a space, a control character or a non-ASCII byte")
    address = text.decode("ascii")
    parts = urllib.parse.urlsplit(address)
    if parts.query or parts.fragment or "?" in address or "#" in address:
        raise Refused("the address carries a query or a fragment")
    if "@" in parts.netloc:
        raise Refused("the address carries credentials")
    host = parts.hostname or ""
    if parts.scheme == "https" and host:
        return text
    if parts.scheme == "http" and loopback(host):
        return text
    raise Refused("the address is not https:// (or http:// to this machine)")


def loopback(host):
    if host == "localhost":
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def written_slot(address):
    return address + b"\0" * (SLOT_LEN - len(address))


def read(path):
    try:
        with open(path, "rb") as source:
            return source.read()
    except OSError as error:
        raise Refused(f"{path}: {error.strerror}") from None


def the_slot(data, path):
    """The unwritten slot's offset; refuses unless it occurs exactly once."""
    found = occurrences(data, MARKER)
    if len(found) != 1:
        headers = len(occurrences(data, HEADER))
        raise Refused(
            f"{path}: the unwritten slot occurs {len(found)} times (its header {headers} times); "
            "it must occur exactly once"
        )
    return found[0]


def check_written(data, address, path):
    """The offset of the written slot; refuses unless the address and its
    padding occur exactly once and no marker is left."""
    if MARKER in data:
        raise Refused(f"{path}: the unwritten slot is still there")
    found = occurrences(data, written_slot(address))
    if len(found) != 1:
        raise Refused(f"{path}: the written address occurs {len(found)} times; it must occur exactly once")
    return found[0]


def command_check(paths):
    for path in paths:
        data = read(path)
        at = the_slot(data, path)
        say(f"{path}: unwritten slot at offset {at:#x}, sha256 {sha256(data)}")


def command_write(address_file, source, target):
    address = read_address(address_file)
    if os.path.lexists(target):
        raise Refused(f"{target} exists; write makes a new file")
    data = read(source)
    at = the_slot(data, source)
    end = at + SLOT_LEN
    written = data[:at] + written_slot(address) + data[end:]
    # In memory first: nothing outside the slot moved.
    if len(written) != len(data) or written[:at] != data[:at] or written[end:] != data[end:]:
        raise Refused(f"{source}: the written copy differs outside the slot")
    mode = os.stat(source).st_mode & 0o7777
    directory = os.path.dirname(os.path.abspath(target))
    handle, temporary = tempfile.mkstemp(prefix=".patch-feed-", dir=directory)
    try:
        with os.fdopen(handle, "wb") as out:
            out.write(written)
            out.flush()
            os.fsync(out.fileno())
        os.chmod(temporary, mode)
        # Read back from disk: what is renamed into place is what was checked.
        back = read(temporary)
        if back != written:
            raise Refused(f"{target}: the file on disk is not what was written")
        if check_written(back, address, target) != at:
            raise Refused(f"{target}: the address landed somewhere other than the slot")
        os.rename(temporary, target)
    except BaseException:
        if os.path.lexists(temporary):
            os.unlink(temporary)
        raise
    say(
        f"{target}: feed address written at offset {at:#x} ({len(address)} of {SLOT_LEN} bytes), "
        f"sha256 {sha256(data)} -> {sha256(written)}"
    )


def command_verify(address_file, paths):
    address = read_address(address_file)
    for path in paths:
        data = read(path)
        at = check_written(data, address, path)
        say(f"{path}: written slot at offset {at:#x}, sha256 {sha256(data)}")


def main(argv):
    parser = argparse.ArgumentParser(prog="patch-feed.py", description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check")
    check.add_argument("files", nargs="+")
    write = commands.add_parser("write")
    write.add_argument("--address-file", required=True)
    write.add_argument("source")
    write.add_argument("target")
    verify = commands.add_parser("verify")
    verify.add_argument("--address-file", required=True)
    verify.add_argument("files", nargs="+")
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "check":
            command_check(arguments.files)
        elif arguments.command == "write":
            command_write(arguments.address_file, arguments.source, arguments.target)
        else:
            command_verify(arguments.address_file, arguments.files)
    except Refused as refused:
        print(f"patch-feed: {refused}", file=sys.stderr, flush=True)
        return 1
    except Exception as error:  # noqa: BLE001 - see below
        # Anything else is reported by its kind only: an unforeseen
        # exception's text or traceback could quote the address.
        print(f"patch-feed: stopped by an unexpected {type(error).__name__}", file=sys.stderr, flush=True)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
