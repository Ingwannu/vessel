"""Fetch Microsoft's HLSL compiler (fxc.exe and d3dcompiler_47.dll) from
the Windows SDK, for building the Windows app on Linux (../patches/README.md
says why it is this compiler).

    python3 -I fetch-fxc.py OUT_DIR

It reads Visual Studio's public release manifest, takes the newest Windows
SDK package, downloads its "Windows SDK for Windows Store Apps Tools" MSI and
the cabinets it names (each checked against the manifest's SHA-256), unpacks
them with msiextract (msitools) into OUT_DIR/sdk, and copies the x64 fxc.exe
and d3dcompiler_47.dll to OUT_DIR. These files are Microsoft's, under the
Windows SDK's licence; they stay on the build machine and nothing of them is
shipped (the app carries only the shader bytes fxc compiles).

Run with `python3 -I` (no current-directory imports); it executes nothing it
downloads.
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import urllib.request

CHANNEL = "https://aka.ms/vs/17/release/channel"
MSI = "Windows SDK for Windows Store Apps Tools-x86_en-us.msi"
AGENT = {"User-Agent": "vessel-release-build"}


def fetch(url, path, sha256=None):
    if not os.path.exists(path):
        request = urllib.request.Request(url, headers=AGENT)
        with urllib.request.urlopen(request) as response:
            data = response.read()
        if sha256 and hashlib.sha256(data).hexdigest().lower() != sha256.lower():
            raise SystemExit(f"SHA-256 mismatch for {url}")
        with open(path + ".part", "wb") as out:
            out.write(data)
        os.replace(path + ".part", path)
    return path


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 -I fetch-fxc.py OUT_DIR")
    out = os.path.abspath(sys.argv[1])
    downloads = os.path.join(out, "downloads")
    os.makedirs(downloads, exist_ok=True)
    channel = json.load(open(fetch(CHANNEL, os.path.join(downloads, "channel.json")), encoding="utf-8-sig"))
    item = next(i for i in channel["channelItems"] if i["id"] == "Microsoft.VisualStudio.Manifests.VisualStudio")
    manifest = json.load(open(fetch(item["payloads"][0]["url"], os.path.join(downloads, "vs.json")), encoding="utf-8-sig"))
    sdks = [p for p in manifest["packages"] if p["id"].startswith(("Win10SDK_10.", "Win11SDK_10."))]
    sdk = max(sdks, key=lambda p: [int(x) for x in p["id"].split("_")[1].split(".")])
    payloads = {pl["fileName"].replace("\\", "/").split("/")[-1]: pl for pl in sdk["payloads"]}
    msi = payloads[MSI]
    msi_path = fetch(msi["url"], os.path.join(downloads, MSI), msi.get("sha256"))
    media = subprocess.run(["msiinfo", "export", msi_path, "Media"], capture_output=True, text=True, check=True).stdout
    for line in media.splitlines()[3:]:
        fields = line.split("\t")
        cabinet = fields[3].lstrip("#") if len(fields) > 3 else ""
        if cabinet:
            payload = payloads[cabinet]
            fetch(payload["url"], os.path.join(downloads, cabinet), payload.get("sha256"))
    unpacked = os.path.join(out, "sdk")
    os.makedirs(unpacked, exist_ok=True)
    subprocess.run(["msiextract", "-C", unpacked, msi_path], check=True, cwd=downloads, stdout=subprocess.DEVNULL)
    bin_root = os.path.join(unpacked, "Program Files", "Windows Kits", "10", "bin")
    version = max(os.listdir(bin_root), key=lambda v: [int(x) for x in v.split(".") if x.isdigit()])
    for name in ("fxc.exe", "d3dcompiler_47.dll"):
        shutil.copy2(os.path.join(bin_root, version, "x64", name), os.path.join(out, name))
    print(f"{sdk['id']}: fxc.exe and d3dcompiler_47.dll ({version}, x64) in {out}")


main()
