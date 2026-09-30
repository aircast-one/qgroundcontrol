#!/usr/bin/env python3
"""Emit the MAVLink message field table the Rust core's inspector decodes payloads with.

Usage: mavlink_field_table.py <xml_dir> <dialect> <output_json>

Each message maps to its name, its fields in XML order (the order mavlink_message_info_t lists
them and the inspector shows them) with type, array length and wire offset, and its instance
field if one is marked.
"""

import json
import sys
from pathlib import Path

from pymavlink.generator import mavparse


def resolve(xml_dir: Path, dialect: str, seen: set) -> list:
    if dialect in seen:
        return []
    seen.add(dialect)
    parsed = mavparse.MAVXML(str(xml_dir / f"{dialect}.xml"), mavparse.PROTOCOL_2_0)
    included = [xml for name in parsed.include for xml in resolve(xml_dir, name.replace(".xml", ""), seen)]
    return included + [parsed]


def table(xmls: list) -> dict:
    messages = {}
    for xml in xmls:
        for message in xml.message:
            if message.id in messages:
                continue
            instance = next((f.name for f in message.fields if getattr(f, "instance", False)), None)
            fields = [[f.name, f.type, f.array_length, f.wire_offset] for f in message.fields]
            messages[message.id] = [message.name, fields, instance]
    return {str(key): messages[key] for key in sorted(messages)}


def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <xml_dir> <dialect> <output_json>", file=sys.stderr)
        sys.exit(1)
    xml_dir, dialect, output = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
    content = json.dumps(table(resolve(xml_dir, dialect, set())), separators=(",", ":")) + "\n"
    if output.exists() and output.read_text() == content:
        print(f"Unchanged: {output}")
        return
    output.write_text(content)
    print(f"Generated {output}")


if __name__ == "__main__":
    main()
