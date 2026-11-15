"""Decode a generated fixture with an independent XML/plist parser.

No file or network input: Rust supplies only synthetic paths over stdin.
Run under Python's isolated, no-site mode so user configuration is ignored.
"""
import json
import plistlib
import sys
import xml.etree.ElementTree as xml

case = json.load(sys.stdin.buffer)
actual = plistlib.loads(case["plist"].encode("utf-8"))
expected = {
    "Label": "vessel.node",
    "ProgramArguments": [case["exe"], "--data-dir", case["directory"], "node", "start"],
    "EnvironmentVariables": {"PATH": case["search"]},
    "RunAtLoad": True,
    "KeepAlive": {"SuccessfulExit": False},
    "StandardOutPath": case["log"],
    "StandardErrorPath": case["log"],
}
assert actual == expected, "plist keys or decoded values changed"


def shape(element):
    # Ignore only container formatting, retaining exact keys/string content.
    text = element.text if element.tag in ("key", "string") else None
    return element.tag, element.attrib, text, [shape(child) for child in element]


reference = plistlib.dumps(expected, sort_keys=False)
assert shape(xml.fromstring(case["plist"])) == shape(xml.fromstring(reference)), (
    "plist structure, duplicate keys or values changed"
)
