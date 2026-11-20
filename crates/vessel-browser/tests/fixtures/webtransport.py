"""Own two local H3 peers for the candidate WebTransport test.

The first peer must answer a real WebTransport CONNECT over QUIC with a
verified TLS certificate. The second peer starts with independently zeroed
counters for the approved browser runner. This script creates no host CA
or network configuration.
"""

import argparse
import asyncio
import contextlib
import datetime
import functools
import ipaddress
import json
import os
from pathlib import Path
import ssl
import sys

import aioquic
from aioquic.asyncio import QuicConnectionProtocol, connect
from aioquic.asyncio.server import QuicServer
from aioquic.h3.connection import H3_ALPN, H3Connection
from aioquic.h3.events import HeadersReceived
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import HandshakeCompleted, ProtocolNegotiated
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec


LOOPBACK = "127.0.0.1"
ROUTE = b"/fixture"


def identity():
    key = ec.generate_private_key(ec.SECP256R1())
    name = x509.Name([x509.NameAttribute(x509.oid.NameOID.COMMON_NAME, "Fixture")])
    now = datetime.datetime.now(datetime.UTC)
    cert = (
        x509.CertificateBuilder()
        .subject_name(name)
        .issuer_name(name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now - datetime.timedelta(hours=1))
        .not_valid_after(now + datetime.timedelta(days=1))
        .add_extension(
            x509.SubjectAlternativeName([x509.IPAddress(ipaddress.ip_address(LOOPBACK))]),
            critical=False,
        )
        .sign(key, hashes.SHA256())
    )
    return cert, key


class Endpoint(QuicConnectionProtocol):
    def __init__(self, quic, stream_handler=None, counters=None):
        super().__init__(quic, stream_handler)
        self.quic = quic
        self.http = None
        self.counters = counters
        self.result = asyncio.get_running_loop().create_future()

    def quic_event_received(self, event):
        if isinstance(event, ProtocolNegotiated):
            self.http = H3Connection(self.quic, enable_webtransport=True)
        if isinstance(event, HandshakeCompleted) and self.counters is not None:
            self.counters["handshakes"] += 1
        if self.http is not None:
            for message in self.http.handle_event(event):
                if isinstance(message, HeadersReceived):
                    self.received_headers(message)

    def received_headers(self, message):
        headers = dict(message.headers)
        if self.counters is None:
            if not self.result.done():
                self.result.set_result(headers.get(b":status"))
            return
        if (
            headers.get(b":method") == b"CONNECT"
            and headers.get(b":protocol") == b"webtransport"
            and headers.get(b":path") == ROUTE
        ):
            self.counters["connects"] += 1
            response = [(b":status", b"200"), (b"sec-webtransport-http3-draft", b"draft02")]
            self.http.send_headers(message.stream_id, response)
            self.transmit()

    async def positive_control(self, port):
        request = [
            (b":method", b"CONNECT"),
            (b":scheme", b"https"),
            (b":authority", f"{LOOPBACK}:{port}".encode("ascii")),
            (b":path", ROUTE),
            (b":protocol", b"webtransport"),
        ]
        self.http.send_headers(self.quic.get_next_available_stream_id(), request)
        self.transmit()
        assert await asyncio.wait_for(self.result, timeout=4) == b"200"


class CountedServer(QuicServer):
    def __init__(self, configuration, counters):
        super().__init__(
            configuration=configuration,
            create_protocol=functools.partial(Endpoint, counters=counters),
        )
        self.counters = counters

    def datagram_received(self, data, address):
        self.counters["datagrams"] += 1
        super().datagram_received(data, address)


@contextlib.asynccontextmanager
async def peer(certificate, key):
    configuration = QuicConfiguration(
        is_client=False, alpn_protocols=H3_ALPN, max_datagram_frame_size=65536,
        certificate=certificate, private_key=key, idle_timeout=5,
    )
    counters = {"datagrams": 0, "handshakes": 0, "connects": 0}
    loop = asyncio.get_running_loop()
    transport, server = await loop.create_datagram_endpoint(
        lambda: CountedServer(configuration, counters), local_addr=(LOOPBACK, 0)
    )
    try:
        yield transport.get_extra_info("sockname")[1], counters
    finally:
        server.close()


async def verify_control(certificate, key):
    client = QuicConfiguration(
        alpn_protocols=H3_ALPN, max_datagram_frame_size=65536, idle_timeout=5,
        cadata=certificate.public_bytes(serialization.Encoding.PEM),
        verify_mode=ssl.CERT_REQUIRED,
    )
    async with peer(certificate, key) as (port, observed):
        async with asyncio.timeout(8):
            async with connect(LOOPBACK, port, configuration=client, create_protocol=Endpoint) as api:
                await api.positive_control(port)
        assert observed["datagrams"] > 0 and observed["handshakes"] == observed["connects"] == 1
        return observed


async def main(args):
    assert aioquic.__version__ == "1.8.0"
    assert os.getuid() == os.getgid() == 1000
    certificate, key = identity()
    report = {"positive_h3": await verify_control(certificate, key)}
    if args.control_only:
        print(json.dumps(report))
        return
    assert args.runner and args.export and args.commit and args.out
    async with peer(certificate, key) as (port, observed):
        environment = dict(os.environ)
        environment["VESSEL_BROWSER_H3_PEER"] = f"{LOOPBACK}:{port}"
        environment["VESSEL_BROWSER_H3_SHA256"] = certificate.fingerprint(hashes.SHA256()).hex()
        process = await asyncio.create_subprocess_exec(
            sys.executable, str(args.runner), str(args.export), args.commit, str(args.out),
            "webtransport", env=environment,
        )
        report["browser_runner_exit"] = await process.wait()
        report["during_browser"] = observed
    assert args.out.is_dir()
    with (args.out / "webtransport.json").open("x") as evidence:
        json.dump(report, evidence, indent=2)
        evidence.write("\n")
    print(json.dumps(report))
    assert report["browser_runner_exit"] == 0
    assert all(count == 0 for count in report["during_browser"].values())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--control-only", action="store_true")
    parser.add_argument("--runner", type=Path)
    parser.add_argument("--export", type=Path)
    parser.add_argument("--commit")
    parser.add_argument("--out", type=Path)
    asyncio.run(main(parser.parse_args()))
