#!/usr/bin/env python3
"""A field the core ADDS is invisible to every other instrument here.

view-fields.py asks whether a key this head DECODES still exists. That direction catches a
removal and cannot catch an addition, so every served-field change that mattered on 2026-09-12
was found by reading a peer's commit or by being told: flightLoop and layers (a structure scan
drew no route), confidence (every detection lost its percentage), rangeText, spinsPropeller.

An earlier attempt compared a model's keys against every quoted literal in the core module that
serves the view. It failed its control: "conf" is a literal in detections.rs because the core
READS that name from upstream on the very line it renames it, so the checker could not tell a key
the core SERVES from one it CONSUMES.

The contract has no such confusion. test/Bridge/fixtures/view-shapes.json records only what was
SERVED, and its _observed list carries every view.path.field ever seen. This reads that.

WHAT IT CANNOT SEE, and the exclusion is structural rather than a bug to fix later: a field inside
an ARRAY element. view.detections records boxes as ["empty"], so x, y, w, h, label, confidence and
target are pinned nowhere, which is exactly why the conf rename was invisible. The recorder cannot
synthesise a detection box because the feed is an external SSE stream. Control: of six fields found
the hard way, this reports five and misses confidence for that reason.

It matches a field by NAME across all of macos/Sources rather than per view, so a name this head
reads for one view counts as read for every view. That UNDER-reports and never invents.

MEASURED, so the obvious fix is not attempted again: 146 of the 730 distinct field names in the
contract appear under more than one view root, so that blind hemisphere is a fifth of the object,
and it has cost something real -- altitudeMetres was served on view.missionItems and unread there
while AdsbModel read a field of the same name from view.adsbTraffic, which is how a launch
position came to be written from the core's fallback altitude (c5daab20c). Scoping the match to
the model head_models.MODELS names for that view root WAS then measured against the whole
contract and reports 223 fields, nearly all of them envelope keys -- class, kind, available,
canUndo -- that a STORE file decodes rather than the model. Models and stores split the decoding
between them, and MODELS maps a view to its model alone, so the strict version is a detector at a
bad ratio. It is not built. The under-reporting stays until something maps a view to every file
that reads it.

Two limits compound: this reports only fields ADDED since SINCE, so a name-collision from before
that line is invisible on both counts.

The MIRROR of this tool -- a head reading a key nothing serves -- has no cheap detector either,
and two designs were measured and rejected rather than guessed at. It is not hypothetical: 0a3b35375
migrated onto a valueMeters that existed only in a peer's uncommitted tree, and the head read nil
for an hour. (1) head-reads.txt cannot carry it: head-reads.py intersects the head's names with the
contract's _observed before writing, so a read of an unserved name is silently DROPPED from the
artefact rather than reported -- checked, the name is absent from that file at both 0a3b35375 and
its revert. (2) Comparing the head's dictionary-subscript keys against the contract at HEAD reports
104 of 459, essentially all legitimate: raw Qt fact keys this head still decodes (minString,
enumStrings, maxIsDefaultForType), write-payload keys (lat, lon, breachReturnPoint), probe argument
keys, Info.plist keys, and the array-element fields the contract records as ["empty"]. valueMeters
would have sat in that list indistinguishable from minString. What separates the two is WHICH path
the dictionary came from, which is the per-view scoping rejected above.

So the guard that contains this class is not a sweep: it is that a write fed by a served number
must REFUSE rather than default. See addBreachReturn, where ?? 0 turned a nil metric altitude into
sea level, and MissionItem.launchAltitudeMetres.

Usage: python3 tools/macos/served-unread.py
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
# The head to check, so this answers the same question of Android as of macOS. There is no
# tools/android/, and sixteen of the seventeen instruments here hardcode macos/Sources -- which is
# why every fine-grained finding in the plan is a macOS finding and Android's had to be derived by
# hand. Android is the head with MORE to measure: 53 raw reads against macOS's 25.
HEADS = {
    "macos": ((ROOT / "macos/Sources",), "*.swift"),
    "android": ((ROOT / "android/app/src/main", ROOT / "android/map-spike/src/main"), "*.kt"),
}
HEAD = sys.argv[1] if len(sys.argv) > 1 else "macos"
if HEAD not in HEADS:
    raise SystemExit(f"unknown head {HEAD!r}: expected one of {', '.join(sorted(HEADS))}")
SOURCES, SUFFIX = HEADS[HEAD]
TREES = " or ".join(str(root.relative_to(ROOT)) for root in SOURCES)
CONTRACT = ROOT / "test/Bridge/fixtures/view-shapes.json"

# The last contract this head has reconciled. Move it forward when the additions
# since it have each been read or accepted -- never to silence a hit.
SINCE = "a1e1d6c7b"

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from head_models import MODELS  # noqa: E402


# Every field the core serves that this head does not read, and why that is right. A view the head
# does not read AT ALL is not listed here -- view-fields.py already carries a reason for each of
# those, and repeating them would be two tables to keep true instead of one.
# An acceptance is a statement about ONE head: "this head draws the text variant instead" is
# true here and false next door. Kept shared, a reason written for one head silences a real
# finding for the other - which is what happened to distanceToVehicle, accepted from
# Android's position while macOS drew neither it nor the text beside it.
ACCEPTED_BY_HEAD = {
    "android": {
        "wholeNumbersOnly": "MEASURED. It is served on view.plan.defaults, which this head draws"
            "nowhere - the plan tab writes altitude and speed through the fact's own path and reads"
            "neither the defaults block nor its bounds. Same reason as decimalPlaces and defaultValue"
            "one field over. It becomes worth reading the day a settings CONTROL carries it, because"
            "an integer setting offered a decimal keyboard invites the value the core just stopped"
            "the vehicle from truncating",
        "showsPacketRadio": "MEASURED. The settings list now comes from view.settings, so the "
            "Packet Radio page is offered and its settings are drawn like any other group. The flag "
            "says a head should draw a BESPOKE radio block - adapter picker and status line - and "
            "there is none here until libusb can enumerate on Android. showsLinks and "
            "showsVideoSources, the two flags that name a block this head does have, are both read",
        "showsAbout": "MEASURED. The About page carries no sections at all, so the flag is the only "
            "thing on it, and this head has no About screen to draw. settingsPages drops a page with "
            "no sections and no block this head draws, which is that page and only that page",
        "decimalPlaces": "MEASURED, and the same split as valueMeters one field over. It is how"
            "MANY places to print, for a head that formats the number itself; this one draws the"
            "valueString and the bound TEXT the core already spelled to that precision. Reading it"
            "would mean formatting a second time from a rule the core has already applied",
        "multiRotor": "MEASURED. See apmFirmware and vtol - the same block on the same view, and"
            "nothing here branches on airframe class. What the plan editor offers comes from"
            "view.plan.actions, which is the core saying what this vehicle accepts",
        "valueMeters": "MEASURED. The metres behind a control's value, for a head that converts "
            "and writes metres itself. Every fact row here draws the fact's OWN valueString beside "
            "the fact's OWN units and writes the typed text back through the fact, so the "
            "conversion happens where the unit is defined - the same argument altitudeMetres is "
            "accepted on, and the defect 8110770d3 removed",
        "apmFirmware": "MEASURED. Nothing in this head branches on airframe or firmware family - "
            "grep for multiRotor, vtol or apmFirmware across both modules returns nothing. What "
            "the plan editor offers is gated by view.plan.actions, which is the core deciding what "
            "this vehicle accepts rather than the head inferring it from a type flag. The day a "
            "screen here needs to draw something differently for a VTOL, it reads these",
        "vtol": "See apmFirmware - the same block, and the same reason",
        "defaultValue": "MEASURED. view.plan.defaults serves the bound and default keys as "
            "numbers beside the text ones. The settings screen moved onto view.settings controls in "
            "the same sweep and reads defaultText, minimumText and maximumText - the SPELLINGS, "
            "formatted to the fact's decimalPlaces. defaultMissionItemAltitude is why: the number is "
            "164.04199475065616 feet and the text is 164.0, so a head that formats the number itself "
            "prints sixteen digits of a converted metre. The numbers are for a head that formats its "
            "own, and this one does not",
        "distanceToVehicle": "MEASURED. This head draws distanceToVehicleText, which is the same "
            "quantity already converted and spelled in the operator's unit. The raw metres and the unit "
            "name beside it are for a head that formats its own numbers, and formatting a second time "
            "here is the defect two-coordinate-serialisers removed",
        "distanceToVehicleMeters": "See distanceToVehicle - the raw quantity behind the text this head "
            "already draws",
        "distanceToVehicleUnits": "See distanceToVehicle - the unit name behind the text this head "
            "already draws",
    },
    "macos": {
        "goneQuiet": "MEASURED, and my first note about this field was wrong twice over. It is on "
            "view.links, not view.vehicleLinks, and the core told me it was not built there -- "
            "links.rs:79 builds it, matching a configured link's name against the quiet list. What "
            "makes it legitimately unread is the next line: gone_quiet feeds the `state` match that "
            "becomes statusLine, and ConnectionsSection already DRAWS statusLine beside every link's "
            "type. Reading the flag as well would be this head re-deriving a sentence the core owns. "
            "The per-link answer for the CONNECTED vehicle is commLost on view.vehicleLinks, which "
            "this head decodes and names in 47b375ff9",
        "layers": "the COUNT of a structure scan's stacked circuits, already on screen as the Layers "
            "fact in the item's own editor, and two stacked circuits are ONE SHAPE on a flat map -- "
            "drawing the ring twice puts identical points on identical points. Android reached the "
            "opposite decision correctly, because their head shows a structure scan's facts nowhere",
        "layerSpanText": "the band a structure scan sweeps, spelled RELATIVE to launch. The item row "
            "already draws the same band as AMSL from altitudeBandText -- 485 m launch, so "
            "\"62.5 m to 87.5 m\" and \"548 m to 572 m AMSL\" are one measurement in two frames. A "
            "SCREENSHOT proved it; the two strings never appear side by side in any payload",
        "rebootRequired": "MEASURED AT THE PRODUCER. The three plan defaults are "
            "defaultMissionItemAltitude, offlineEditingCruiseSpeed and offlineEditingHoverSpeed, and "
            "none of the three declares a reboot or restart key in App.SettingsGroup.json -- so "
            "vehicleRebootRequired and applicationRestartRequired are false by construction and this, "
            "their OR, is false with them. A notice drawn from any of the three could never appear. "
            "Control against reading that absence as universal: 3 of the 35 facts in that same group "
            "DO declare one. The mechanism is not missing either -- SettingsControl carries "
            "restartNotices and the Settings window draws them -- so this is the fact never saying "
            "so, not the head being unable to hear it",
        "vehicleRebootRequired": "See rebootRequired -- the same three facts and the same check",
        "applicationRestartRequired": "See rebootRequired -- the same three facts and the same check",
        "defaultText": "MEASURED and deliberately undrawn. It is the value a fact would return TO, "
            "and nothing in this head offers a reset: the plan defaults are three plain value fields "
            "and the settings pages edit in place. A row that showed what it would revert to without "
            "offering the revert describes an action the operator cannot take from that screen -- the "
            "console case, where a sentence implying something impossible is worse than silence. It "
            "becomes worth reading the day one of those screens grows a Reset control",
        "distanceToVehicle": "MEASURED. This head draws distanceToVehicleText in the Fly panel, "
            "which is the same quantity already converted and labelled in the operator's unit. This "
            "is the NUMBER behind it, for a head that formats its own; formatting it a second time "
            "here is the defect two-coordinate-serialisers removed",
        "distanceToVehicleMeters": "See distanceToVehicle - the raw metres behind the text this head "
            "draws, in a fixed unit the panel never shows",
        "distanceToVehicleUnits": "See distanceToVehicle - the unit name behind the text this head "
            "draws, already inside it",
    },
}

ACCEPTED = {
    "autoDisconnect": "whether the vehicle drops its link by itself after contact is lost. It is "
        "a SETTING, and this head neither draws it nor may write it -- FirmwareUpgrade.qml is the "
        "only place upstream that touches it, and it WRITES it as part of a flow this head does "
        "not have. Drawing a control's state with no control is the unread-field debt removed in "
        "c485b1aac",
    "rangeText": "the guided slider's two ends as one combined string. This head draws them as "
        "SEPARATE labels at opposite ends of the track, and the dragged value between them can "
        "only be spelled here, so adopting it would put two precision rules on one control. The "
        "core agreed and declined to serve separate ends",
}
ACCEPTED = {**ACCEPTED, **ACCEPTED_BY_HEAD[HEAD]}


# Whether a view is drawn at all is MEASURED per head rather than read from view-fields.py's
# UNDRAWN table, which states the macOS head's reasons and was silencing this question for both.
# view.adsbTraffic is the case that proves it: that table calls it not built, which is true there
# and false here - TrafficView.kt has drawn it since 2026-09-13 - so every field of it this head
# failed to read was being excused by another head's reason. Same shape as the acceptance table
# split earlier: one table cannot answer a question whose answer differs between the two trees.
def views_named_here():
    text = "\n".join(
        path.read_text(errors="replace")
        for root in SOURCES
        for path in sorted(root.rglob(SUFFIX))
    )
    return set(re.findall(r'"(view\.[A-Za-z0-9_]+)', text))


DRAWN_HERE = views_named_here()


def read_here(view):
    return view.split(".")[0] + "." + view.split(".")[1] in DRAWN_HERE if view.count(".") >= 1 else False


def observed_fields(text=None, drawn_only=True):
    contract = json.loads(text) if text else json.loads(CONTRACT.read_text())
    seen = {}
    for path in contract.get("_observed", []):
        view, _, field = path.rpartition(".")
        if not (view and field.isidentifier()) or (drawn_only and not read_here(view)):
            continue
        seen.setdefault(field, set()).add(view)
    return seen


def contract_at(revision):
    import subprocess
    shown = subprocess.run(["git", "show", f"{revision}:test/Bridge/fixtures/view-shapes.json"],
                           cwd=ROOT, capture_output=True, text=True)
    return shown.stdout if shown.returncode == 0 else None


def names_the_head_uses():
    text = "\n".join(
        path.read_text(errors="replace")
        for root in SOURCES
        for path in sorted(root.rglob(SUFFIX))
    )
    return set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', text)) | set(
        re.findall(r"\b([a-z][A-Za-z0-9_]*)\b", text))


used = names_the_head_uses()
before = contract_at(SINCE)
if before is None:
    print(f"  CANNOT COMPARE the contract against {SINCE}, which git does not have",
          file=sys.stderr)
    sys.exit(1)

# BOTH sides come from git, and the working copy is never read. Four sessions share this checkout,
# so the fixture on disk routinely holds a re-recording somebody has not committed -- and this tool
# compared that against a committed revision, which makes every in-flight field look newly served.
# It reported six such fields in one run, three of them (parametersReason, parametersText,
# wholeNumbersOnly) naming nothing that exists in any commit. Two heads would then have migrated
# onto them, which is exactly what 0a3b35375 cost an hour for.
# The question this tool asks is "what does the core serve", and in a shared checkout the only
# honest answer is what has landed.
latest = contract_at("HEAD")
if latest is None:
    print("  CANNOT READ the contract at HEAD, which is the only version this compares against",
          file=sys.stderr)
    sys.exit(1)

now, then = observed_fields(latest), observed_fields(before)
added = sorted((field, sorted(views)) for field, views in now.items()
               if field not in then and field not in used and field not in ACCEPTED)

# Against every served field, not only the ones in views this head draws: a field that moved
# into a view this head does not draw is still served, and calling it gone would delete a
# reason that is about to be needed again.
served_anywhere = observed_fields(drawn_only=False)
stale = [f"{field!r} is accepted and the core no longer serves it"
         for field in sorted(ACCEPTED) if field not in served_anywhere]

# The other way an acceptance rots, and the one the NOT SERVED check cannot see: the field is
# still served AND this head now reads it. The reason has been answered by the work rather than
# by the core, so the entry excuses nothing -- until somebody deletes the read, when it silences
# exactly the signal it was written to make explicit. An acceptance that is doing no work is not
# harmless; it is a guard pre-disarmed for a defect nobody has written yet.
answered = [f"{field!r} is accepted as undrawn and this head now reads it"
            for field in sorted(ACCEPTED) if field in used]

for why in stale:
    print(f"  NOT SERVED {why}", file=sys.stderr)
for why in answered:
    print(f"  ANSWERED {why}, so delete the acceptance rather than leaving it to excuse a "
          f"future removal -- CHECK THE READ IS ON THE ACCEPTED VIEW FIRST, because this matches "
          f"by NAME and a fifth of the contract's names appear under more than one view root",
          file=sys.stderr)
for field, views in added:
    where = ", ".join(views[:3]) + (" and more" if len(views) > 3 else "")
    print(f"  NEWLY SERVED {field!r} appeared in the contract for {where} since {SINCE}, and no "
          f"file under {TREES} names it. Read it, or accept it with the reason it stays "
          f"undrawn and move SINCE forward", file=sys.stderr)

print(f"compared {len(now)} served field names against the contract at {SINCE}: "
      f"{len(now) - len(then)} added since, {len(added)} of them named nowhere in this head, "
      f"{len(ACCEPTED)} accepted with a reason")
sys.exit(1 if added or stale or answered else 0)
