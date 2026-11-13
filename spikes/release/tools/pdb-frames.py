"""Name the frames of a Windows panic recorded by vessel-app-log.

    python3 -I tools/pdb-frames.py <Vessel.exe> <vessel_app.pdb> <vessel.log>

The log's panic line has frames="exe+0x<rva> ...". Each RVA is looked up
among the .pdb's public symbols (the build keeps each label's .pdb in the
symbols directory, build.sh's keep_symbols): the nearest function at or
below it, with the offset into it. The release profile builds without
debug info, so the .pdb has no line tables for the app's own code;
std's frames are named with file and line by `llvm-symbolizer --obj=<exe>
--relative-address`. Names are Rust's v0 mangling (`rustfilt` demangles).

Needs llvm-pdbutil on PATH. Standard library only otherwise; reads the
exe's section table itself.
"""

import bisect
import re
import struct
import subprocess
import sys


def sections(exe_path):
    data = open(exe_path, "rb").read()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional = struct.unpack_from("<H", data, pe + 20)[0]
    table = pe + 24 + optional
    return [struct.unpack_from("<I", data, table + i * 40 + 12)[0] for i in range(count)]


def publics(pdb_path, section_rvas):
    dump = subprocess.run(
        ["llvm-pdbutil", "dump", "-publics", pdb_path],
        check=True,
        capture_output=True,
        text=True,
        errors="replace",
    ).stdout
    found, name = [], None
    for line in dump.splitlines():
        symbol = re.search(r"S_PUB32 \[size = \d+\] `(.*)`", line)
        if symbol:
            name = symbol.group(1)
            continue
        place = re.search(r"flags = ([^,]*), addr = (\d+):(\d+)", line)
        if place and name is not None:
            section, offset = int(place.group(2)), int(place.group(3))
            if "function" in place.group(1) and 1 <= section <= len(section_rvas):
                found.append((section_rvas[section - 1] + offset, name))
            name = None
    found.sort()
    return found


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    exe, pdb, log = sys.argv[1:]
    symbols = publics(pdb, sections(exe))
    starts = [start for start, _ in symbols]
    text = open(log, encoding="utf-8", errors="replace").read()
    for panic in re.findall(r'frames="([^"]*)"', text):
        print("panic:")
        for frame in panic.split():
            if not frame.startswith("exe+"):
                print(f"  {frame}  (another module)")
                continue
            rva = int(frame[4:], 16)
            index = bisect.bisect_right(starts, rva) - 1
            if index < 0:
                print(f"  {frame}  ?")
            else:
                start, name = symbols[index]
                print(f"  {frame}  {name} +{rva - start:#x}")


if __name__ == "__main__":
    main()
