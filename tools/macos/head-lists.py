#!/usr/bin/env python3
"""Pin a hand-written list in the head against the code it has to agree with.

A list someone maintains by hand rots the moment the thing it mirrors changes, and nothing
says so. These are the ones where drift is silent rather than loud.

SetupPage.bespoke names every page VehicleSetupWindow's content switch has a case for. A name
in the switch but not the list is a page that is built and never offered; a name in the list
but not the switch is a page that is offered and opens on nothing.

HostNotice.Kind decodes the tokens contract.rs declares as the served domain. This one drifts
SILENTLY IN ONE DIRECTION BY DESIGN: an unrecognised kind decodes to .unknown and is still
DRAWN, because the core meant to say something and this head not knowing the word is no reason
to lose the sentence. So a fourth kind renders unclassified and nothing anywhere says a fourth
kind exists. The served domain is the only thing that can tell me, which is why the core added
it rather than a view.
"""
import re
import sys

ROOT = __file__.rsplit("/tools/", 1)[0]


def read(path):
    with open(f"{ROOT}/{path}") as handle:
        return handle.read()


def switch_cases(path, opening, closing):
    """The case labels of one switch, from its opening line to its default.

    Assumes the switch has no nested switch with its own default before it; one that did would
    truncate the scan. The failure is loud - names go missing and this exits non-zero.
    """
    text = read(path)
    start = text.index(opening)
    end = text.index(closing, start)
    return set(re.findall(r'case "([^"]+)"', text[start:end]))


def swift_set(path, anchor):
    text = read(path)
    start = text.index(anchor)
    end = text.index("]", text.index("[", start))
    return set(re.findall(r'"([^"]+)"', text[start:end]))


declared = swift_set("macos/Sources/SetupPageModel.swift", "static let bespoke")
built = switch_cases("macos/Sources/VehicleSetupWindow.swift",
                     "private var content: some View {", "default:")

if not declared or not built:
    print("found no names on one side, which is a broken reader rather than a clean result",
          file=sys.stderr)
    sys.exit(1)

offered_but_blank = sorted(declared - built)
built_but_hidden = sorted(built - declared)
for name in offered_but_blank:
    print(f"SetupPage.bespoke lists {name!r}, which the content switch has no case for: "
          f"choosing it opens on nothing", file=sys.stderr)
for name in built_but_hidden:
    print(f"the content switch draws {name!r}, which SetupPage.bespoke omits: it is built and "
          f"never offered", file=sys.stderr)

if offered_but_blank or built_but_hidden:
    sys.exit(1)

def rust_array(path, name):
    text = read(path)
    start = text.index(f"{name}: [&str;")
    return set(re.findall(r'"([^"]+)"', text[text.index("[", text.index("=", start)):
                                             text.index("]", text.index("=", start)) + 1]))


def swift_cases(path, anchor):
    text = read(path)
    start = text.index(anchor)
    return set(re.findall(r'case "([^"]+)"', text[start:text.index("default:", start)]))


served = rust_array("core-rs/src/contract.rs", "NOTICE_KINDS")
decoded = swift_cases("macos/Sources/HostNoticeModel.swift", "init(_ token: String?)")

if not served or not decoded:
    print("found no tokens on one side, which is a broken reader rather than a clean result",
          file=sys.stderr)
    sys.exit(1)

undrawn = sorted(served - decoded)
invented = sorted(decoded - served)
for token in undrawn:
    print(f"the core serves notice kind {token!r} and HostNotice.Kind has no case for it: it "
          f"decodes to .unknown and is DRAWN unclassified, which is the one drift this head "
          f"cannot notice on its own", file=sys.stderr)
for token in invented:
    print(f"HostNotice.Kind decodes {token!r}, which contract.rs does not serve: a case no "
          f"reply can select", file=sys.stderr)

if undrawn or invented:
    sys.exit(1)

# A settings path spelled in the head is a hand-written reference to a name a tree this head
# does not own declares. Rename the fact upstream and the read stops resolving -- silently, the
# way every list in this file rots: the Remote Support host field goes blank, the map-centre
# toggle stops reflecting, the packet radio device name disappears, and no build breaks.
# src/Settings/*.json is the producer and is shared three ways, so this READS it and never edits.
import json
import pathlib

groups = sorted(pathlib.Path(f"{ROOT}/src/Settings").glob("*.json"))
# Keyed on the GROUP as well as the fact. A bare leaf name is not unique across these files --
# the core measured `enabled` declared in both PacketRadio and Viewer3D -- so checking only that
# the name exists somewhere passes a path pointing at the wrong group, which is the defect this
# is for: settings.videoSettings.deviceName would resolve to nothing while `deviceName` exists
# in PacketRadio. The group segment maps to a file by dropping the trailing "Settings" and
# capitalising, which holds for all six paths this head spells.
declared = {}
unreadable = []
for group in groups:
    try:
        declared[group.name.replace(".SettingsGroup.json", "")] = {
            entry.get("name") for entry in
            json.loads(group.read_text()).get("QGC.MetaData.Facts", [])
            if isinstance(entry, dict)}
    except (ValueError, OSError) as why:
        unreadable.append(f"{group.name}: {why}")

spelled = set()
for source in sorted(pathlib.Path(f"{ROOT}/macos/Sources").glob("*.swift")):
    spelled |= set(re.findall(r'"settings\.([A-Za-z0-9_]+)\.([A-Za-z0-9_]+)"', source.read_text()))

# Zero paths is a broken reader, not a clean head. This head demonstrably spells settings paths,
# so an empty set means the pattern stopped matching -- and then `missing` is empty too and the
# line below reports success over a subject it never collected. Measured: breaking the pattern
# prints "0 settings paths name a fact" and exits 0.
if groups and not spelled:
    print("found no settings paths in macos/Sources, so whether they still name a fact is "
          "UNMEASURED rather than clean -- this head does spell them, so zero is this reader "
          "failing rather than an answer", file=sys.stderr)
    sys.exit(1)

if unreadable or not groups:
    for why in unreadable:
        print(f"cannot read {why}", file=sys.stderr)
    print(f"read {len(groups)} settings group files, so whether the head's settings paths still "
          f"name a fact is UNMEASURED rather than clean", file=sys.stderr)
    sys.exit(1)

missing = []
for group, fact in sorted(spelled):
    stem = group[:-8] if group.endswith("Settings") else group
    stem = stem[:1].upper() + stem[1:]
    if stem not in declared:
        missing.append(f"the head spells settings.{group}.{fact} and src/Settings has no "
                       f"{stem}.SettingsGroup.json: the group segment names no file")
    elif fact not in declared[stem]:
        elsewhere = sorted(g for g, facts in declared.items() if fact in facts)
        aside = f" -- {fact!r} is declared in {', '.join(elsewhere)}" if elsewhere else ""
        missing.append(f"the head spells settings.{group}.{fact} and {stem}.SettingsGroup declares "
                       f"no fact of that name{aside}: the read resolves to nothing and no build "
                       f"says so")
for why in missing:
    print(why, file=sys.stderr)
if missing:
    sys.exit(1)

print(f"head lists pinned: SetupPage.bespoke matches {len(built)} content switch cases, "
      f"HostNotice.Kind matches {len(served)} served notice kinds, {len(spelled)} settings paths "
      f"name a fact in the group they point at, across {len(groups)} group files declaring "
      f"{sum(len(v) for v in declared.values())} facts")
