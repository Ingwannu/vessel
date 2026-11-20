"""Serve a fixed DNS transition in a disposable Linux network namespace."""

import argparse
import asyncio
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys


PUBLIC = "203.0.113.2"
PRIVATE = "127.0.0.1"
REBIND = b"rebind.fixture.test"
MIXED = b"mixed.fixture.test"
RESOLVER = "nameserver 127.0.0.1\noptions timeout:1 attempts:1\n"
UID = GID = 1000


def checked_setup(resolver):
    if os.getuid() != 0 or not sys.flags.isolated:
        raise RuntimeError("use the isolated namespace launcher with Python -I")
    for namespace in ("net", "mnt"):
        if os.readlink(f"/proc/self/ns/{namespace}") == os.readlink(f"/proc/1/ns/{namespace}"):
            raise RuntimeError("fixture refuses the host namespaces")
    if any(" shared:" in row.split(" - ", 1)[0]
           for row in Path("/proc/self/mountinfo").read_text().splitlines()):
        raise RuntimeError("mount propagation must be private")
    links = json.loads(subprocess.check_output(["/usr/sbin/ip", "-j", "link", "show"]))
    if [link["ifname"] for link in links] != ["lo"]:
        raise RuntimeError("fixture requires a fresh loopback-only network namespace")
    if resolver.read_text() != RESOLVER:
        raise RuntimeError("unexpected resolver configuration")
    subprocess.run(["/usr/sbin/ip", "link", "set", "dev", "lo", "up"], check=True)
    # A route reaches the synthetic peer without assigning a public address
    # to a local interface. The policy's interface observation stays real.
    subprocess.run(["/usr/sbin/ip", "route", "add", "local", f"{PUBLIC}/32", "dev", "lo"], check=True)
    subprocess.run(["/usr/bin/mount", "--bind", str(resolver), "/etc/resolv.conf"], check=True)
    subprocess.run(["/usr/bin/mount", "-o", "remount,bind,ro", "/etc/resolv.conf"], check=True)
    listener = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    listener.bind((PRIVATE, 53))
    os.setgroups([])
    os.setresgid(GID, GID, GID)
    os.setresuid(UID, UID, UID)
    if os.getresuid() != (UID,) * 3 or os.getresgid() != (GID,) * 3:
        raise RuntimeError("fixture could not drop all privileged identities")
    return listener


class Resolver(asyncio.DatagramProtocol):
    def __init__(self):
        self.counts = {"public_answer": 0, "rebound_answer": 0, "mixed_answer": 0}

    def connection_made(self, transport):
        self.transport = transport

    def datagram_received(self, data, source):
        if source[0] != PRIVATE or len(data) < 17 or len(data) > 512:
            return
        name, question_end = self.question(data)
        if not question_end:
            return
        kind, resource_class = struct.unpack_from("!HH", data, question_end - 4)
        if resource_class != 1:
            return
        answers = self.answers(name) if kind == 1 else []
        flags = 0x8180 if name in (REBIND, MIXED) else 0x8183
        response = data[:2] + struct.pack("!HHHHH", flags, 1, len(answers), 0, 0)
        response += data[12:question_end]
        for address in answers:
            response += b"\xc0\x0c" + struct.pack("!HHIH", 1, 1, 0, 4) + socket.inet_aton(address)
        self.transport.sendto(response, source)

    @staticmethod
    def question(data):
        if data[4:6] != b"\x00\x01":
            return b"", 0
        labels = []
        offset = 12
        while offset < len(data) - 4:
            count = data[offset]
            offset += 1
            if count == 0:
                return b".".join(labels).lower(), offset + 4
            if count > 63 or offset + count > len(data) - 4:
                return b"", 0
            labels.append(data[offset:offset + count])
            offset += count
        return b"", 0

    def answers(self, name):
        if name == REBIND:
            if self.counts["public_answer"] == 0:
                self.counts["public_answer"] += 1
                return [PUBLIC]
            self.counts["rebound_answer"] += 1
            return [PRIVATE]
        if name == MIXED:
            self.counts["mixed_answer"] += 1
            return [PUBLIC, PRIVATE]
        return []


async def fixture_peer(reader, writer, counts, route):
    try:
        line = await asyncio.wait_for(reader.readline(), timeout=2)
        parts = line.split()
        if route == "allowed" and len(parts) == 3 and parts[1] == b"/first":
            counts["first"] += 1
            body = b"<link rel=icon href=data:,>Fixture public address reached"
        else:
            counts[route + "_other"] += 1
            body = b"fixture unexpected dial"
        writer.write(
            b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html\r\n"
            + f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
            + body
        )
        await writer.drain()
    finally:
        writer.close()
        await writer.wait_closed()


async def observe(args, dns_socket):
    loop = asyncio.get_running_loop()
    dns, resolver = await loop.create_datagram_endpoint(Resolver, sock=dns_socket)
    counts = {"first": 0, "allowed_other": 0, "refused_other": 0}
    try:
        allowed = await asyncio.start_server(
            lambda reader, writer: fixture_peer(reader, writer, counts, "allowed"), PUBLIC, 0
        )
        async with allowed:
            port = allowed.sockets[0].getsockname()[1]
            refused = await asyncio.start_server(
                lambda reader, writer: fixture_peer(reader, writer, counts, "refused"), PRIVATE, port
            )
            async with refused:
                if args.control_only:
                    for wanted in (PUBLIC, PRIVATE):
                        records = await loop.getaddrinfo(
                            REBIND.decode(), port, family=socket.AF_INET, type=socket.SOCK_STREAM
                        )
                        if [record[4][0] for record in records] != [wanted]:
                            raise RuntimeError("controlled DNS transition failed")
                    print(json.dumps({"resolver": resolver.counts, "uid": os.getuid(), "gid": os.getgid()}))
                    return
                environment = {"VESSEL_BROWSER_DNS_PORT": str(port), "LANG": "C.UTF-8"}
                process = await asyncio.create_subprocess_exec(
                    "/usr/bin/python3", str(args.runner), str(args.export), args.commit,
                    str(args.out), "dns", env=environment,
                )
                code = await process.wait()
        report = {
            "uid": os.getuid(), "gid": os.getgid(), "resolver": resolver.counts,
            "connections": counts, "browser_runner_exit": code,
        }
        with (args.out / "resolver.json").open("x") as evidence:
            json.dump(report, evidence, indent=2)
            evidence.write("\n")
        print(json.dumps(report))
        if code or counts != {"first": 1, "allowed_other": 0, "refused_other": 0}:
            raise RuntimeError("actual browser route or destination check failed")
        if not all(resolver.counts.values()):
            raise RuntimeError("the browser did not exercise the complete DNS transition")
    finally:
        dns.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resolver", required=True, type=Path)
    parser.add_argument("--control-only", action="store_true")
    parser.add_argument("--runner", type=Path)
    parser.add_argument("--export", type=Path)
    parser.add_argument("--commit")
    parser.add_argument("--out", type=Path)
    config = parser.parse_args()
    if not config.control_only and not all((config.runner, config.export, config.commit, config.out)):
        parser.error("the full fixture requires the runner and exact export")
    asyncio.run(observe(config, checked_setup(config.resolver.resolve(strict=True))))
