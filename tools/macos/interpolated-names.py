#!/usr/bin/env python3
"""Pin the head's hand-written path segments against the Q_PROPERTY that answers them.

Mission.swift and GeoTag.swift interpolate these names into bridge paths. A misspelling
is invisible from the consumer side: a read resolves to null and draws a default, and a
write resolves to nothing and simply does not happen, with no error in between. The core
never names either list, so nothing but this checks them.

It reads DECLARATIONS, not the running app's reflection. A Q_PROPERTY that exists here
can still be unreachable at runtime for a reason this cannot see; a name absent here is
wrong for certain.
"""
import re
import sys

ROOT = __file__.rsplit("/tools/", 1)[0]


def read(path):
    with open(f"{ROOT}/{path}") as handle:
        return handle.read()


def properties(*headers):
    return {
        match.group(2): match.group(0)
        for header in headers
        for match in re.finditer(r"Q_PROPERTY\(\s*([\w:*<> ]+?)\s+(\w+)\s+([^)]*)\)", read(header))
    }


def swift_names(path, anchor):
    """The names in a Swift literal: a dictionary's keys, or an array's elements."""
    text = read(path)
    start = text.index(anchor)
    depth = 0
    for end in range(text.index("[", start), len(text)):
        depth += text[end] == "["
        depth -= text[end] == "]"
        if not depth:
            break
    literal = text[start:end + 1]
    keyed = re.findall(r'"([^"]+)"\s*:', literal)
    return keyed or re.findall(r'"([^"]+)"', literal)


def component_classes(*headers):
    """VehicleComponent subclasses declared in these headers."""
    return {
        match.group(1)
        for header in headers
        for match in re.finditer(r"class\s+(\w+)\s*:\s*public\s+VehicleComponent\b", read(header))
    }


def check_classes(what, names, headers):
    declared = component_classes(*headers)
    where = " or ".join(headers)
    bad = [f"{what}: {name!r} is not a VehicleComponent subclass in {where}" for name in names
           if name not in declared]
    for line in bad:
        print(line, file=sys.stderr)
    return not bad


def check(what, names, headers, needs_write):
    declared = properties(*headers)
    where = " or ".join(headers)
    bad = []
    for name in names:
        if name not in declared:
            bad.append(f"{what}: {name!r} is not a Q_PROPERTY in {where}")
        elif needs_write and " WRITE " not in declared[name]:
            bad.append(f"{what}: {name!r} in {where} has no WRITE, so the head's write is discarded")
    for line in bad:
        print(line, file=sys.stderr)
    return not bad


lists = swift_names("macos/Sources/ItemFactModel.swift", "static let lists")
labels = swift_names("macos/Sources/GeoTag.swift", "static let labels")
survey = swift_names("macos/Sources/SurveyStatsModel.swift", "static let properties")
sensors = swift_names("macos/Sources/VehicleComponentModel.swift", "static let sensorClasses")

if not lists or not labels or not survey or not sensors:
    print("found no names to check, which is a broken reader rather than a clean result",
          file=sys.stderr)
    sys.exit(1)

ok = check("ItemFact.lists", lists, ["src/MissionManager/SimpleMissionItem.h"], False)
ok &= check("GeoTagStore.labels", labels, ["src/AnalyzeView/GeoTagController.h"], True)
# A survey's shot count lives on the transect base class and its flown distance on the complex
# base above it, so this one spans two headers.
ok &= check("SurveyWatch.properties", survey,
            ["src/MissionManager/TransectStyleComplexItem.h",
             "src/MissionManager/ComplexMissionItem.h"], False)

# Class names rather than properties: the head recognises the sensors component by class because
# its name is translated, and a hand-written list of C++ class names rots exactly as silently as a
# hand-written list of property names.
ok &= check_classes("VehicleComponentInfo.sensorClasses", sensors,
                    ["src/AutoPilotPlugins/PX4/SensorsComponent.h",
                     "src/AutoPilotPlugins/APM/APMSensorsComponent.h"])

if ok:
    print(f"interpolated names pinned: {', '.join(lists + labels + survey + sensors)}")
sys.exit(0 if ok else 1)
