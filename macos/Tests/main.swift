import Foundation

var failures = 0

func expect(_ actual: String, _ expected: String, _ label: String) {
    if actual != expected {
        FileHandle.standardError.write("FAIL \(label): got \(actual.debugDescription), want \(expected.debugDescription)\n".data(using: .utf8)!)
        failures += 1
    }
}

func expect(_ condition: Bool, _ label: String) {
    if !condition {
        FileHandle.standardError.write("FAIL \(label)\n".data(using: .utf8)!)
        failures += 1
    }
}

let controlJson: [String: Any] = [
    "path": "settings.appSettings.audioMuted", "name": "audioMuted", "label": "Audio muted",
    "control": "toggle", "value": true as NSNumber, "valueString": "true", "display": "On",
    "units": "", "readOnly": false as NSNumber, "restartNotices": [] as [String],
    "options": [], "decimalPlaces": 0 as NSNumber,
]
guard let toggle = SettingsControl(controlJson) else {
    fatalError("the core's control shape must decode")
}
expect(toggle.kind == .toggle, "a toggle control is a toggle")
expect(toggle.boolValue, "and carries its value")
expect(toggle.minimum == nil && toggle.maximum == nil,
       "a toggle has no bounds, and the core sends null rather than a type extreme")

let choice = SettingsControl([
    "path": "settings.unitsSettings.speedUnits", "name": "speedUnits", "label": "Speed",
    "control": "choice", "value": 1 as NSNumber, "valueString": "1",
    "options": [["label": "Feet/second", "raw": "0"],
                ["label": "Metres/second", "raw": "1"]],
])
expect(choice?.kind == .choice, "a choice control is a choice")
expect(choice?.options.count == 2, "with the core's options")
expect(choice?.options.last?.raw ?? "", "1",
       "each option carries the raw value to write, so the head never indexes by position")
expect(SettingsControl(["path": "g.c", "control": "choice",
                        "options": [["label": "A", "raw": 0 as NSNumber]]])?.options.isEmpty == true,
       "the core sends raw as TEXT, because a raw can be 2.5 or a word; reading it as a number "
       + "dropped every option and would have drawn a choice with an empty picker")

let bounded = SettingsControl([
    "path": "g.n", "name": "n", "control": "number", "value": 3 as NSNumber,
    "minimum": 0 as NSNumber, "maximum": 5.0e9 as NSNumber,
])
expect(bounded?.maximum == 5.0e9,
       "a real five-billion ceiling survives, where this head used to discard any bound above "
       + "a billion as a type extreme and quietly drop a genuine limit")

let unbounded = SettingsControl(["path": "g.m", "name": "m", "control": "number",
                                 "value": 3 as NSNumber])
expect(unbounded?.minimum == nil && unbounded?.maximum == nil,
       "and an unbounded fact has no bounds at all, because the core decides that from "
       + "minIsDefaultForType rather than from the magnitude")

expect(SettingsControl(["path": "g.x"]) == nil,
       "a control with no kind is dropped rather than rendered as a text field")
expect(SettingsControl(["control": "toggle"]) == nil,
       "and one with no path is dropped, because the path is what a write goes to")
expect(SettingsControl(["path": "g.y", "control": "invented"])?.kind == .unknown,
       "a control kind this head does not know is unknown, and falls through to a plain field")

let arming = SettingsControl([
    "path": "vehicle.parameter.ARMING_CHECK", "name": "ARMING_CHECK", "control": "bitmask",
    "value": 82 as NSNumber, "valueString": "82",
    "bits": [["label": "All", "raw": "1", "set": false as NSNumber],
             ["label": "Barometer", "raw": "2", "set": true as NSNumber],
             ["label": "Compass", "raw": "4", "set": false as NSNumber],
             ["label": "GPS lock", "raw": "16", "set": true as NSNumber],
             ["label": "Parameters", "raw": "64", "set": true as NSNumber]],
])
expect(arming?.kind == .bitmask,
       "a parameter an operator sets bit by bit is a bitmask, not a number; it used to fall "
       + "through to a text field that accepted typing and silently discarded it")
expect(arming?.drawsBits == true, "and it draws a box per bit")
expect(arming?.bits.filter(\.set).map(\.label).joined(separator: ",") ?? "",
       "Barometer,GPS lock,Parameters",
       "the core says which bits are set and this head does not re-derive them from the value")

expect(arming.map { $0.toggling($0.bits[2], on: true) } == 86,
       "ticking Compass adds its bit to the value the core reported, rather than replacing it")
expect(arming.map { $0.toggling($0.bits[3], on: false) } == 66,
       "and clearing GPS lock removes only that bit, leaving the other two set")
expect(arming.map { $0.toggling($0.bits[1], on: true) } == 82,
       "ticking a bit that is already set changes nothing, so a redraw cannot corrupt the value")

expect(SettingsControl(["path": "g.b", "control": "bitmask",
                        "bits": [["label": "None", "raw": "0", "set": false as NSNumber]]])?
    .bits.isEmpty == true,
       "a zero bit is dropped: it can never be set and its box could never change the value, "
       + "so it would be a control that does nothing")
expect(SettingsControl(["path": "g.b", "control": "bitmask",
                        "bits": [["label": "Odd", "raw": "2.5", "set": false as NSNumber]]])?
    .bits.isEmpty == true,
       "and a bit whose raw is not a whole number is dropped rather than toggling nothing")

let signed = SettingsControl([
    "path": "vehicle.parameter.SERVO_OPTS", "name": "SERVO_OPTS", "control": "bitmask",
    "value": -128 as NSNumber, "valueString": "-128",
    "bits": [["label": "Reverse", "raw": "1", "set": false as NSNumber],
             ["label": "Top", "raw": "-128", "set": true as NSNumber]],
])
expect(signed?.bits.count == 2,
       "the ArduPilot metadata casts each bit to the parameter's own type, so an int8 carries "
       + "its top bit as -128; dropping it as unparseable would lose a real bit")
expect(signed.map { $0.toggling($0.bits[1], on: false) } == 0,
       "clearing a negative bit clears exactly that bit, because the value sign-extends the "
       + "same way the bit does")
expect(signed.map { $0.toggling($0.bits[0], on: true) } == -127,
       "and setting an ordinary bit alongside it leaves the top bit where it was")

let emptyMask = SettingsControl(["path": "g.b", "name": "b", "control": "bitmask",
                                 "value": 7 as NSNumber, "valueString": "7"])
expect(emptyMask?.kind == .bitmask && emptyMask?.drawsBits == false,
       "a bitmask the core sent no usable bits for shows its value instead of an editor with "
       + "nothing in it; an absent bits list must not draw an empty box list")

func link(_ overrides: [String: Any]) -> LinkConfig? {
    var json: [String: Any] = ["index": 0 as NSNumber, "type": "tcp"]
    overrides.forEach { json[$0.key] = $0.value }
    return LinkConfig(json)
}

expect(link(["connected": true as NSNumber])?.connected == true,
       "the core decides whether a link is connected, from the LinkInterface it sees as a child "
       + "rather than from a value that reads null on every connected link")
expect(link(["connected": false as NSNumber])?.connected == false, "and when it is not")
expect(link(["typeLabel": "TCP"])?.typeLabel ?? "", "TCP", "the type label is the core's")
expect(link(["type": "logReplay", "typeLabel": "Log Replay"])?.typeLabel ?? "", "Log Replay",
       "a log replay link renders readably")
expect(link(["type": "other", "typeLabel": "AirLink"])?.typeLabel ?? "", "AirLink",
       "and a link type neither side has heard of takes the name QGC gives its settings page")

expect(link(["displaySummary": "No host set"])?.displaySummary ?? "", "No host set",
       "a hostless TCP link says so, because \"TCP \u{00B7} :5760\" hid that it cannot connect")
expect(link(["displaySummary": "UDP port 14550"])?.displaySummary ?? "", "UDP port 14550",
       "and a UDP link keeps its summary, having no host to be missing")

expect(link(["port": 14550 as NSNumber])?.port == 14550,
       "the port is whichever of port and localPort the link actually has, which the core picks; "
       + "reading only port reported every UDP link as port 0")
expect(link(["port": 14550 as NSNumber])?.portText ?? "", "14550",
       "and it reads as itself where the operator edits it")
expect(link([:])?.baud == nil,
       "baud is a property of SerialLink alone, so a UDP or TCP link has none. a9b4e2a53 made "
       + "the core say so rather than answer zero, and decoding it into an Int put the zero back")
expect(link([:])?.baudText ?? "x", "",
       "an absent rate selects nothing in the picker rather than a rate of 0, which is not a "
       + "rate the core offers and not one any link runs at")
expect(link([:])?.portText ?? "x", "",
       "and a log replay, which has neither a port nor a local port, offers an empty field to "
       + "type into rather than a port numbered zero")
expect(link(["logFileName": "flight.tlog", "filename": "/tmp/flight.tlog"])?.logFileName ?? "",
       "flight.tlog", "and the log file shows its last path component, not the whole path")

expect(link(["editing": "hostAndPort"])?.editing == .hostAndPort, "TCP edits host and port")
expect(link(["editing": "portOnly"])?.editing == .portOnly, "UDP edits only its local port")
expect(link(["editing": "serial"])?.editing == .serial, "serial edits device and baud")
expect(link(["editing": "logFile"])?.editing == .logFile, "a replay link chooses a file")
expect(link(["editing": "none"])?.editing == LinkConfig.Editing.none,
       "and a mock link has nothing to edit")
expect(link(["editing": "somethingNew"])?.editing == .unknown,
       "an editing mode this head does not know edits nothing, rather than falling into the "
       + "host-and-port form and offering fields the link has no use for")

expect(link(["connected": true as NSNumber, "heardVehicle": false as NSNumber])?.health == .waiting,
       "an open link that has heard nothing is waiting, not connected; binding a UDP socket "
       + "cannot fail, so the head painted a green dot for a port with nothing on it")
expect(link(["connected": true as NSNumber, "heardVehicle": true as NSNumber])?.health == .heard,
       "a link is only healthy once a MAVLink packet has actually been decoded on it")
expect(link(["connected": false as NSNumber, "heardVehicle": true as NSNumber])?.health == .closed,
       "and a closed link is closed however much it heard before, so a stale heardVehicle "
       + "cannot keep the dot lit")
expect(link([:])?.health == .closed,
       "a link the core said nothing about is closed rather than assumed live")
expect(link(["statusLine": "Waiting for the vehicle"])?.statusLine ?? "",
       "Waiting for the vehicle",
       "and the sentence is the core's, so both heads say the same thing")

expect(LinkConfig(["type": "tcp"]) == nil, "a link with no index is dropped")
expect(LinkConfig(["index": 0 as NSNumber]) == nil, "and one with no type is dropped")
expect(LinkConfig.list(nil).isEmpty, "no answer is no links")

let vibrationJson: [String: Any] = [
    "available": true as NSNumber, "units": "", "scaleMaximum": 90.0 as NSNumber,
    "warningLevel": 30.0 as NSNumber, "dangerLevel": 60.0 as NSNumber,
    "worst": "danger",
    "clipCounts": [0 as NSNumber, 2 as NSNumber, 0 as NSNumber],
    "clipping": true as NSNumber,
    "axes": [
        ["axis": "x", "label": "X", "value": 5.0 as NSNumber,
         "fraction": 0.0555 as NSNumber, "severity": "normal"],
        ["axis": "y", "label": "Y", "value": 65.0 as NSNumber,
         "fraction": 0.7222 as NSNumber, "severity": "danger"],
        ["axis": "z", "label": "Z", "value": 35.0 as NSNumber,
         "fraction": 0.3888 as NSNumber, "severity": "warning"],
    ],
]
let vibration = VibrationReading(vibrationJson, connected: true)
expect(vibration.axes.count == 3, "each axis the core reports is carried across")
expect(vibration.axes.map(\.label).joined(separator: ","), "X,Y,Z",
       "with the core's display label, so no head upper-cases an axis name itself")
expect(vibration.worst == .danger,
       "one bad axis makes the whole reading dangerous, and the core decides which is worst")
expect(vibration.axes[1].severity == .danger, "each bar takes its own colour from its own severity")
expect(vibration.dangerLevel == 60 && vibration.warningLevel == 30 && vibration.scaleMaximum == 90,
       "the thresholds the bars and the scale are drawn against come from the core, which is "
       + "where ArduPilot and PX4's flight guidance now lives rather than in two heads")
expect(VehicleSetupText.connectPrompt(for: "vibration"),
       "Connect a vehicle to see its vibration.",
       "one place spells the sentence a page shows when there is nothing to read from. Four "
       + "sites wrote it separately and a fifth would have drifted; waiting(), filtered(), the "
       + "console and vibration all take it from here now")
expect(VehicleSetupText.waiting(connected: false, for: "console output")
           == MavlinkConsole.none.emptyText,
       "so the console's disconnected line and the helper's cannot say different things")
expect(VibrationReading([:], connected: false).emptyText,
       "Connect a vehicle to see its vibration.",
       "with nothing connected there is no vehicle to be silent, and the page said \u{201C}This "
       + "vehicle is not reporting vibration\u{201D} -- asserting an aircraft that is not there, "
       + "the same shape as the readiness pill in 7b015c270")
expect(VibrationReading([:], connected: true).emptyText,
       "This vehicle is not reporting vibration.",
       "and a vehicle that is present and silent still reads as one, which is a measurement "
       + "rather than a regime that does not apply")
expect(vibration.clipping,
       "the core decides whether the accelerometer clipped and this head carries the answer; it "
       + "derives nothing from clipCounts, which is why the two are set independently above")
expect(VibrationReading(["available": true as NSNumber,
                         "clipCounts": [0 as NSNumber, 2 as NSNumber, 0 as NSNumber]],
                        connected: true).clipping
       == false,
       "so counts without the flag read as no clipping. That default is permissive and only the "
       + "required-keys row for view.vibration keeps it unreachable; the decode does not")

let quiet = VibrationReading(["available": false as NSNumber,
                              "axes": [["axis": "x", "label": "X"]]], connected: true)
expect(quiet.worst == nil,
       "a vehicle reporting no level has no worst level, which is not the same as normal")
expect(quiet.axes[0].value == nil && quiet.axes[0].severity == nil,
       "and an axis with no reading draws no bar rather than a bar at zero, which would look "
       + "like a perfectly still airframe")
expect(!VibrationReading.unavailable.available, "the unavailable reading reports itself as such")
expect(VibrationReading.Severity("molten") == nil,
       "a severity this head does not know is no severity, not the reassuring one")

let outsideEnum = Parameter(name: "ACRO_RP_RATE_TC", componentId: 1, json: [
    "enumIndex": 5, "valueString": "0.00", "units": "s",
    "enumStrings": ["Very Soft", "Soft", "Medium", "Crisp", "Very Crisp", "Unknown: 0"]])
expect(outsideEnum.value, "0.00", "a value outside its enum shows the number")

let insideEnum = Parameter(name: "ACRO_RP_EXPO", componentId: 1, json: [
    "enumIndex": 2, "valueString": "0.30", "enumStrings": ["Low", "Med", "High"]])
expect(insideEnum.value, "High", "a value inside its enum shows the label")

let noEnum = Parameter(name: "ACRO_BAL_ROLL", componentId: 1, json: [
    "enumIndex": -1, "valueString": "1.0"])
expect(noEnum.value, "1.0", "a plain numeric parameter shows its value")

expect(Parameter(name: "ATC_ANG_PIT_P", componentId: 1, json: [:]).group, "ATC", "group is the prefix")
expect(Parameter(name: "SCHED_LOOP_RATE", componentId: 1, json: [:]).group, "SCHED", "group stops at the first underscore")
expect(Parameter(name: "FORMAT", componentId: 1, json: [:]).group, "FORMAT", "an ungrouped parameter is its own group")
expect(Parameter(name: "ATC_ANG_PIT_P", componentId: 1, json: [:]).path,
       "vehicle.parameterManager.getParameter(1,ATC_ANG_PIT_P)", "path calls getParameter")

let sensorItems: [Any] = [
    ["name": "GPS", "state": "unhealthy", "label": "Fault"],
    ["name": "Gyro", "state": "healthy", "label": "Healthy"],
    ["name": "Logging", "state": "disabled", "label": "Not enabled"],
]
let listedSensors = SensorHealth.list(sensorItems)
expect(listedSensors.map(\.name).joined(separator: ","), "GPS,Gyro,Logging",
       "the core has already put faults first, and the head keeps that order rather than re-sorting")
expect(listedSensors[0].state == .unhealthy, "an enabled sensor that is unhealthy is a fault")
expect(listedSensors[2].state == .disabled,
       "and a disabled sensor is not a fault, which is why Geofence and Logging do not cry wolf")
expect(listedSensors[2].label, "Not enabled", "each row carries the core's own wording")

expect(SensorHealth(["name": "GPS"]) == nil,
       "a sensor with no state is dropped rather than drawn as healthy")
expect(SensorHealth(["state": "healthy"]) == nil,
       "and one with no name is dropped, because the name is what the row is keyed by")
expect(SensorHealth(["name": "GPS", "state": "molten"])?.state == SensorHealth.State.unknown,
       "a state this head does not know reads as unknown, not as the reassuring one")
expect(SensorHealth.list(nil).isEmpty, "no answer is no sensors")

let setupPage: [String: Any] = [
    "page": "Safety", "firmware": "apm", "available": true as NSNumber,
    "sections": [
        ["title": "Failsafe", "note": "What the vehicle does when it loses the transmitter.",
         "controls": [["path": "p.FS_THR_ENABLE", "name": "FS_THR_ENABLE", "control": "choice",
                       "label": "Throttle failsafe", "valueString": "1", "display": "Enabled",
                       "options": [["label": "Disabled", "raw": "0"],
                                   ["label": "Enabled", "raw": "1"]]]]],
        ["title": "Return to Launch", "note": "",
         "controls": [["path": "p.RTL_ALT", "name": "RTL_ALT", "control": "number",
                       "label": "Return altitude", "valueString": "1500", "units": "cm"]]],
    ],
]
let safetySections = SettingsSection.list(setupPage["sections"])
expect(safetySections.map(\.title).joined(separator: ","), "Failsafe,Return to Launch",
       "the core lists only the sections this vehicle has parameters for, in its own order")
expect(safetySections[0].controls.count == 1,
       "a setup section carries its controls directly, where a settings page nests them in "
       + "subsections; one decode reads both")
expect(safetySections[0].controls[0].options.count == 2,
       "and a parameter's options come through as the core's label and raw text pairs")
expect(safetySections[1].controls[0].units, "cm",
       "with the parameter's own units, which the row shows beside the field")

let nav = PageSelection(owner: "vehicleSetup", pages: ["Sensors", "Safety", "Parameters"])
expect(nav.page, "Sensors", "selection starts on the first page")
expect(nav.identifier, "vehicleSetup.pages", "each window gets its own probe key")
expect((nav.probeInvoke(action: "select", args: ["page": "Safety"])["ok"] as? Bool) == true,
       "selecting a known page succeeds")
expect(nav.page, "Safety", "the selection actually moved")
expect((nav.probeInvoke(action: "select", args: ["page": "Nope"])["ok"] as? Bool) == false,
       "an unknown page is refused")
expect(nav.page, "Safety", "a refused selection leaves the page alone")
expect((nav.probeInvoke(action: "wat", args: [:])["ok"] as? Bool) == false, "unknown actions are refused")

expect(String(FlightModePosition.all.count), "6", "ArduPilot maps six switch positions")
expect(FlightModePosition.all.first!.pwmRange, "up to 1230", "the first band is open-ended below")
expect(FlightModePosition.all.last!.pwmRange, "1750 and above", "the last band is open-ended above")
expect(FlightModePosition.all.map(\.parameter).joined(separator: ","),
       "FLTMODE1,FLTMODE2,FLTMODE3,FLTMODE4,FLTMODE5,FLTMODE6", "positions map to FLTMODE1..6 in order")

expect(FlightModePosition.present(in: ["FLTMODE1", "FLTMODE3"]).map(\.index).map(String.init).joined(separator: ","),
       "1,3", "only reported positions appear")
expect(FlightModePosition.present(in: []).isEmpty, "a vehicle without them shows no positions")

let waypoint = MissionItem(view: [
    "index": 1, "sequence": 1, "name": "Waypoint",
    "coordinate": ["latitude": -35.3629, "longitude": 149.165],
    "altitude": 50.0, "altitudeUnits": "ft", "altitudeText": "50.0 ft",
    "specifiesAltitude": true], selected: -1)
expect(waypoint.altitudeText, "50.0 ft",
       "the core spells the altitude and the head shows what it said. The number and the unit "
       + "travel alongside because the row's editor needs them apart, but nothing here writes a "
       + "measurement any more")
expect(waypoint.positionText, "-35.362900, 149.165000", "position formats to six decimals")
expect(waypoint.hasPosition, "a waypoint the core gave a coordinate has a position")

let start = MissionItem(view: [
    "index": 0, "sequence": 0, "name": "Mission Start", "kind": "settings",
    "coordinate": ["latitude": -35.36, "longitude": 149.16], "altitude": 584.09,
    "altitudeText": "584 m"], selected: 0)
expect(start.altitudeText, "584 m",
       "and the settings row is spelled by the same formatter as the summary strip above it, "
       + "which is what stopped it reading 584.1 m beside 584 m to 660 m")

let startInFeet = MissionItem(view: [
    "index": 0, "sequence": 0, "name": "Mission Start", "altitude": 1916.3,
    "altitudeUnits": "ft", "altitudeText": "1916 ft"], selected: -1)
expect(startInFeet.altitudeText, "1916 ft", "and an operator on feet is told feet")
expect(start.isSelected,
       "the field is named for what it IS: it decodes the core's \"selected\" and every "
       + "consumer means the selected row, so it was called isCurrent for no reason the code "
       + "supports -- loadSelectedFacts says so in its own name and the probe hands it back "
       + "out as \"selected\". MissionManager's currentIndex is a DIFFERENT thing, the item "
       + "the aircraft is flying to, and a field called isCurrent that means selection is what "
       + "would make adopting the real one dangerous. "
       + "The editor's selection is one index on the view, not a flag on each item -- the core "
       + "stopped serving a per-item \"current\" because it invited a head to draw a marker "
       + "meaning the aircraft is there, and this head went on reading the absent key and "
       + "selected nothing at all")
expect(!startInFeet.isSelected, "and an item whose index is not the selected one is not selected")

expect(waypoint.index == 1, "an item remembers the list position its bridge path needs")
expect(waypoint.specifiesAltitude, "a waypoint's altitude is editable")
expect(!start.specifiesAltitude, "an item that does not specify altitude is not editable")

let bare = MissionItem(view: ["index": 2, "sequence": 2, "name": "Delay"], selected: -1)
expect(!bare.hasPosition, "an item the core gave no coordinate has no position")
expect(bare.positionText, "—", "and shows nothing rather than a false one")
expect(bare.altitudeText, "—",
       "an item the core gave no altitude shows a dash, which is the core's own answer for one")

func checkMapFraming() {
    let canberra = MapFrame(latitudes: [-35.363262, -35.362900],
                            longitudes: [149.165237, 149.165000])
    expect(abs(canberra.centreLatitude - -35.363081) < 1e-5, "mission centre sits between the waypoints")
    expect(abs(canberra.centreLongitude - 149.1651185) < 1e-5, "mission centre longitude sits between the waypoints")
    expect(canberra.latitudeDelta < 0.01, "two close waypoints frame tightly, not to a hemisphere")
    expect(canberra.longitudeDelta < 0.01, "and so does the longitude span")

    let single = MapFrame(latitudes: [-35.36], longitudes: [149.16])
    expect(single.latitudeDelta == MapFrame.minimumDelta, "one waypoint still gets a minimum span")
    expect(single.longitudeDelta == MapFrame.minimumDelta, "one waypoint still gets a minimum longitude span")
    expect(abs(single.centreLatitude - -35.36) < 1e-9, "and is centred on that waypoint")
    expect(abs(single.centreLongitude - 149.16) < 1e-9, "on both axes")

    let empty = MapFrame(latitudes: [], longitudes: [])
    expect(empty.centreLatitude == 0 && empty.centreLongitude == 0,
           "no waypoints centres on the origin rather than on NaN")
    expect(empty.latitudeDelta == MapFrame.minimumDelta
           && empty.longitudeDelta == MapFrame.minimumDelta,
           "and spans the minimum rather than the whole world")

    let bogus = MapFrame(latitudes: [-35.36, .nan, 1000], longitudes: [149.16, .infinity])
    expect(abs(bogus.centreLatitude - -35.36) < 1e-9, "a NaN or out-of-range latitude cannot drag the frame")
    expect(abs(bogus.centreLongitude - 149.16) < 1e-9, "a non-finite longitude cannot drag the frame")
    expect(bogus.latitudeDelta == MapFrame.minimumDelta
           && bogus.longitudeDelta == MapFrame.minimumDelta,
           "and what is left is one point, framed to the minimum span")

    let dateline = MapFrame(latitudes: [-16.5, -16.5], longitudes: [179.9, -179.9])
    expect(abs(abs(dateline.centreLongitude) - 180) < 1e-9,
           "two waypoints a fifth of a degree apart across the antimeridian are centred between "
           + "them, not on the far side of the planet: longitude wraps, so min and max are the "
           + "two ends of the SHORT arc and averaging them pointed at the Gulf of Guinea")
    expect(dateline.longitudeDelta < 1,
           "and the span is the short way round; taking max minus min called them 359.8 degrees "
           + "apart, which MissionMap then clamped to half the world")

    let pacific = MapFrame(latitudes: [0, 0], longitudes: [-170, 170])
    expect(abs(abs(pacific.centreLongitude) - 180) < 1e-9,
           "the same holds for a wider straddle, where the empty gap is the one over Africa")
    expect(abs(pacific.longitudeDelta - 20 * MapFrame.padding) < 1e-9,
           "and the span is twenty degrees, not three hundred and forty")

    let spread = MapFrame(latitudes: [0, 0, 0], longitudes: [-10, 0, 10])
    expect(abs(spread.centreLongitude) < 1e-9,
           "a set that does not straddle is unchanged, because its widest gap is the one that "
           + "runs the long way round behind it")
    expect(abs(spread.longitudeDelta - 20 * MapFrame.padding) < 1e-9,
           "and so is its span")

    expect(abs(MapFrame.normalisedLongitude(181) - -179) < 1e-9, "181 east is 179 west")
    expect(abs(MapFrame.normalisedLongitude(-181) - 179) < 1e-9, "and 181 west is 179 east")
    expect(abs(MapFrame.normalisedLongitude(149.16) - 149.16) < 1e-9,
           "an ordinary longitude is left alone")

    let everywhere = MapFrame(latitudes: [-90, 90], longitudes: [-180, 180])
    expect(everywhere.latitudeDelta <= 180 && everywhere.longitudeDelta <= 360,
           "a frame can no longer describe a span larger than the planet, which is what the "
           + "padding used to produce and what an isUsable nothing ever called used to describe")
}

checkMapFraming()

func checkFenceGeometry() {
    let polygon = FenceShape([
        "index": 0 as NSNumber, "path": "plan.geoFenceController.polygons.0",
        "shape": "polygon", "inclusion": true as NSNumber,
        "kindText": "Keep-in polygon", "detailText": "4 vertices \u{00B7} 85268 m\u{00B2}",
        "usable": true as NSNumber,
        "vertices": [["latitude": -35.3607 as NSNumber, "longitude": 149.1612 as NSNumber],
                     ["latitude": -35.3607 as NSNumber, "longitude": 149.1698 as NSNumber]],
        "framing": [["latitude": -35.3607 as NSNumber, "longitude": 149.1612 as NSNumber],
                    ["latitude": -35.3607 as NSNumber, "longitude": 149.1698 as NSNumber]],
    ])
    expect(polygon?.kindText ?? "", "Keep-in polygon",
           "the core names the fence, so both heads call a keep-in the same thing")
    expect(polygon?.shapeText ?? "", "Polygon", "the row title reads from the shape word")
    expect(polygon?.id ?? "", "plan.geoFenceController.polygons.0",
           "the path is the identity, so the head no longer subtracts one array's length from "
           + "the other's to tell a circle from a polygon")

    let circle = FenceShape([
        "index": 0 as NSNumber, "path": "plan.geoFenceController.circles.0",
        "shape": "circle", "inclusion": false as NSNumber,
        "kindText": "Keep-out circle", "detailText": "250 ft radius",
        "centre": ["latitude": -35.36 as NSNumber, "longitude": 149.16 as NSNumber],
        "centreText": "-35.360000, 149.160000",
        "radius": 250.0 as NSNumber, "radiusUnits": "ft", "usable": true as NSNumber,
        "framing": [["latitude": -35.3611 as NSNumber, "longitude": 149.1587 as NSNumber],
                    ["latitude": -35.3589 as NSNumber, "longitude": 149.1613 as NSNumber]],
    ])
    expect(circle?.isCircle == true, "a circle knows it is one from the core's shape word")
    expect(circle?.detailText ?? "", "250 ft radius",
           "and names the units its radius fact came in, which the core writes")
    expect(circle?.rowDetail ?? "x", "",
           "its row shows no detail, because the radius is already the row's value")
    expect(circle?.framing.count == 2,
           "the framing box is the core's, computed from the radius rather than here")
    expect(circle?.usable == true, "and the core decides whether it has enough geometry to draw")

    expect(FenceShape(["path": "p", "shape": "circle"]) == nil, "a shape with no index is dropped")
    expect(FenceShape(["index": 0 as NSNumber]) == nil,
           "and one with no shape word is dropped rather than guessed at")
    expect(FenceShape.list(nil).isEmpty, "no answer is no fences")

    expect(FenceShape(["index": 0 as NSNumber, "path": "p", "shape": "circle"])?.inclusion
           == false,
           "a fence that does not say which kind it is reads as KEEP-OUT. Mistaking an exclusion "
           + "zone for a boundary to stay inside plans a mission into forbidden airspace, and the "
           + "core made the same choice in fences.rs; this head defaulted the other way")

    expect(FactWrite.readOnly.contains("not written"),
           "a read-only fact refuses the write and says so. The only thing stopping one was "
           + "disabled() on four fields, and a display cannot keep a value off the wire")
}
checkFenceGeometry()

func checkAFenceSaysWhichSideIsSafe() {
    func fence(_ inclusion: Bool) -> FenceShape? {
        FenceShape(["index": 0 as NSNumber, "shape": "circle", "path": "p",
                    "inclusion": inclusion as NSNumber, "kindText": inclusion ? "Keep-in circle"
                        : "Keep-out circle", "detailText": "", "usable": true as NSNumber,
                    "centre": ["latitude": 47.4 as NSNumber, "longitude": 8.5 as NSNumber],
                    "centreText": "", "radius": 120.0 as NSNumber, "radiusUnits": "m"])
    }
    expect(fence(true)?.boundary ?? "", FenceShape.keepIn,
           "a keep-in fence and a keep-out fence are opposite instructions -- one says the "
           + "aircraft must stay inside, the other that it must stay out -- and the map drew "
           + "both in the same orange, because the renderer took an inclusion flag and then "
           + "ignored it")
    expect(fence(false)?.boundary ?? "", FenceShape.keepOut,
           "so the two must not share a value here either; a boundary that answered the same "
           + "for both would let the map collapse them again with nothing going red")
    expect(FenceShape.keepIn != FenceShape.keepOut,
           "and the two names are distinct, without which both assertions above pass while "
           + "saying nothing")

    let enforced = FirmwareFence(["radiusMetres": 300.0 as NSNumber, "radiusText": "300 m"])
    expect(enforced?.boundary ?? "", FenceShape.enforced,
           "the fence the FIRMWARE enforces is a THIRD kind, not a keep-in drawn differently. "
           + "It was drawn in systemRed -- and red is keep-OUT in this head and #FF3B30 on "
           + "Android, arrived at independently. A circular firmware fence is a keep-IN: the "
           + "vehicle stops you LEAVING it. So an operator read an exclusion zone around home. "
           + "Inverted, not merely distinct, and Android found it by reading 0e4df8eda")
    expect(FenceShape.enforced != FenceShape.keepIn
           && FenceShape.enforced != FenceShape.keepOut,
           "so it takes neither plan colour. The deciding argument is the AFFORDANCE neither "
           + "head named first: a plan fence can be dragged, resized and deleted; the firmware "
           + "ring cannot be touched, and both heads exclude it from their hit tests. Identical "
           + "appearance PROMISES an interaction that does nothing -- the operator drags it and "
           + "the map ignores them. Both heads now draw it violet and dashed")
}

checkAFenceSaysWhichSideIsSafe()

func checkTheFenceTheFirmwareEnforces() {
    func enforced(_ overrides: [String: Any]) -> FirmwareFence? {
        FirmwareFence(["radiusMetres": 300.0 as NSNumber, "radiusText": "300 m"]
            .merging(overrides) { _, override in override })
    }

    let placed = enforced(["centre": ["latitude": 47.397 as NSNumber,
                                      "longitude": 8.546 as NSNumber]])
    expect(placed?.radiusText ?? "", "300 m",
           "the vehicle enforces a circular fence from its own parameters -- FENCE_RADIUS on "
           + "ArduPilot -- and nothing in this head could see it. An operator can draw a two "
           + "kilometre survey inside a three hundred metre fence, upload it cleanly and find "
           + "out in the air")
    expect(placed?.drawable == true, "with a centre it can be drawn on the map")

    let unplaced = enforced([:])
    expect(unplaced != nil,
           "a radius with NO centre is still a fence and still reported: the core builds the "
           + "centre from vehicle.homePosition, so a vehicle that has reported its fence "
           + "parameter but not yet its home sends a radius and a null centre. Refusing the "
           + "whole thing there would hide the number the operator most needs")
    expect(unplaced?.drawable == false,
           "but it cannot be drawn, and drawing it at a guessed centre would put a circle "
           + "somewhere the firmware does not enforce one")
    expect(unplaced?.rowDetail ?? "", FirmwareFence.unplacedDetail,
           "so the row says the vehicle has not reported where from yet, rather than showing "
           + "the same sentence as a placed one")
    expect(placed?.rowDetail ?? "", FirmwareFence.detail,
           "and a placed one says the vehicle enforces it, which is the whole reason it is "
           + "drawn differently from a fence someone drew in this plan")

    expect(FirmwareFence(["radiusMetres": 0.0 as NSNumber]) == nil,
           "a radius of zero is how this reads when no fence is set, and a zero-radius circle "
           + "would draw a dot the operator would take for a fence")
    expect(FirmwareFence(["radiusText": "300 m"]) == nil, "and a text with no measure is none")
    expect(FirmwareFence(nil) == nil, "as is the null the core sends with no vehicle at all")
}

checkTheFenceTheFirmwareEnforces()

func checkTheBatteryIsSpelledOnce() {
    let pack = BatteryReading(["voltage": 15.8 as NSNumber, "voltageText": "15.80V",
                               "currentText": "12.50A", "percentText": "90%"])
    expect(pack?.voltageText ?? "", "15.80V",
           "the core spells each measure from the vehicle's own Fact, units included, and the "
           + "Fly view already drew that string through secondaryText. This panel took the raw "
           + "numbers and re-spelled them -- \"15.80 V\" -- so ONE battery was written two ways "
           + "in one app, and String(format:) is locale-independent, so a build whose Fact says "
           + "15,80V would still have printed a full stop here")

    let unspelled = BatteryReading(["voltage": 15.8 as NSNumber])
    expect(unspelled?.voltageText ?? "", BatteryReading.unreported,
           "and with no spelling the head now says nothing rather than spelling its own. The "
           + "formatter it used to fall back to is DELETED: keeping it would have left one "
           + "locale-dependent string in a panel whose other rows come from the core, which is "
           + "the disagreement this set out to remove")
    expect(unspelled?.available == true,
           "the reading is still a reading -- the voltage gate is the number, not the string, "
           + "so a pack the core has not spelled yet is present rather than missing")
}

checkTheBatteryIsSpelledOnce()

func checkACoordinateIsSpelledOneWay() {
    expect(GeoPoint.text(-35.363262, 149.165237), "-35.363262, 149.165237",
           "five places in this head spelled a coordinate with their own copy of "
           + "\"%.6f, %.6f\" -- a rally point, a mission item, the launch position and two "
           + "rows in the fly view. They agreed, which is why nothing caught it, and they "
           + "would have drifted the first time one of them changed")
    expect(GeoPoint.text(47.0, 8.0), "47.000000, 8.000000",
           "and it matches the string the core already spells for a fence centre, which "
           + "fences.rs builds with format!(\"{lat:.6}, {lon:.6}\"). Both are "
           + "locale-independent BY CONSTRUCTION -- Rust's {:.6} and Swift's String(format:) "
           + "both emit a full stop -- so unlike the battery this was never a disagreement "
           + "waiting to happen, and the head keeps spelling coordinates rather than asking "
           + "the core to serve a text for every one")
    expect(GeoPoint.text(nil, 8.0), "\u{2014}", "a coordinate half missing is not a position")
    expect(GeoPoint.text(.nan, 8.0), "\u{2014}",
           "and a nan reads as absent rather than as the word nan on the screen")
}

checkACoordinateIsSpelledOneWay()

func checkTheCapabilityKeepsItsThirdState() {
    expect(FenceSupport(answer: true as NSNumber).offers,
           "a vehicle the core says accepts a geofence offers one")
    expect(FenceSupport(answer: false as NSNumber).refusal(servedReason: "") ?? "",
           FenceSupport.unsupportedRefusal, "one that refuses says so")
    expect(FenceSupport(answer: false as NSNumber)
               .refusal(servedReason: "This link accepts neither a geofence nor rally points.") ?? "",
           "This link accepts neither a geofence nor rally points.",
           "and when the core has spelled WHY, that sentence wins over the head's own. plan.rs "
           + "distinguishes four cases this head cannot -- neither, fence only, rally only, and a "
           + "vehicle that has not answered -- and its own test says why the wording matters: "
           + "GeoFenceController::supported is a capability bit AND maxProtoVersion >= 200, so a "
           + "false can mean the vehicle lacks the feature OR that the link speaks MAVLink 1. "
           + "This head said \"This vehicle does not accept a geofence\", picking one of the two "
           + "causes without reading either, and an operator who believes it goes looking at the "
           + "wrong end. The fallback now says \"link\" too, for the same reason")
    expect(FenceSupport(answer: nil).refusal(servedReason: "ignored") ?? "",
           FenceSupport.unreadRefusal,
           "and a vehicle that has NOT YET ANSWERED is a third state, not a refusal. The core "
           + "returns null until capabilitiesKnown is true -- QGC's capabilityBits DEFAULT to "
           + "fence and rally, and its supported() ignores whether they were ever reported, so "
           + "the raw controller property this head used to read says yes to a vehicle that has "
           + "said nothing. Collapsing null to false would spell an unanswered vehicle as a "
           + "refusing one, which is the sentence the core's own test warns about")
    expect(FenceSupport(answer: nil).offers == false,
           "an unanswered capability offers nothing either, so nothing is drawn on a guess")
}

checkTheCapabilityKeepsItsThirdState()

func checkAMeasureNeverReadsMinusZero() {
    expect(Measure.settled("-0.0"), "0.0",
           "the core strips the sign when every printed digit is a zero -- read.rs wraps its "
           + "number in settled() -- and this head's copy of format_measure did not. A vehicle "
           + "on the ground reports a relative altitude a hair below zero, so a rally point or a "
           + "launch altitude read \"-0.0 m\" here and \"0.0 m\" from the core. On an altitude "
           + "that reads as BELOW THE LAUNCH POINT at a glance")
    expect(Measure.settled("-0"), "0", "the guard is on the printed digits, not the input")
    expect(Measure.settled("-0.00"), "0.00", "at any number of places")
    expect(Measure.settled("-1.0"), "-1.0",
           "and a number with a non-zero digit anywhere keeps its sign, which is what stops the "
           + "guard from eating a real reading")

    let range = GuidedRange(["available": true as NSNumber, "minimum": -5.0 as NSNumber,
                             "maximum": 100.0 as NSNumber, "initial": 0.0 as NSNumber,
                             "label": "Height", "unit": "m"])
    expect(range?.text(-0.04) ?? "", "0.0 m",
           "the guided slider is the ONE place this head still spells a measurement, because "
           + "the value is DRAGGED and the core spells no guided reading. It keeps its own "
           + "precision rule -- a decimal below ten, none above -- and shares the guard")

    let grounded = GuidedRange(["available": true as NSNumber, "minimum": -0.03 as NSNumber,
                                "maximum": 60.0 as NSNumber, "initial": 0.0 as NSNumber,
                                "label": "Height", "unit": "m"])
    expect(grounded?.text(grounded?.minimum ?? 0) ?? "", "0.0 m",
           "and the slider's END LABELS are guarded too, which is a separate requirement from "
           + "the dragged value and the one Android shipped broken. The core serves minimum as "
           + "the LESSER of the configured floor and the vehicle's CURRENT altitude, and a "
           + "grounded vehicle reads a hair below launch on baro drift -- so the end label was "
           + "\"-0.0 to 60.0 m\" on a control an operator uses to command a CLIMB. It is safe "
           + "here only because the guard lives inside text(), which all three drawing sites "
           + "call; had it been applied at the value's call site the ends would have escaped it")
    expect(grounded?.text(grounded?.maximum ?? 0) ?? "", "60 m",
           "and the far end keeps the whole-number rule above ten")
}

checkAMeasureNeverReadsMinusZero()

func checkMyLocationTrustsTheCoresGate() {
    func fix(_ overrides: [String: Any]) -> GcsFix? {
        GcsFix(["usable": true as NSNumber, "fix": "gps", "source": "internalGps",
                "latitude": 47.397 as NSNumber, "longitude": 8.546 as NSNumber]
            .merging(overrides) { _, override in override })
    }

    expect(fix([:])?.point != nil,
           "a usable fix centres the map where the operator is standing")

    expect(fix(["usable": false as NSNumber])?.point == nil,
           "and an UNUSABLE one does not, however well-formed its numbers are. This head used "
           + "to read positionManager.gcsPosition raw and judge it with MapCentre.usable, "
           + "which checks only that the coordinate is valid and is not null island. The core "
           + "also refuses a fix coarser than 100 m or older than 5 s, so the head would have "
           + "centred My Location on a reading the core calls unusable -- a map jumping to a "
           + "point two kilometres from where the operator stands, with nothing on screen "
           + "saying the fix was poor")

    expect(fix(["latitude": 0.0 as NSNumber, "longitude": 0.0 as NSNumber])?.point == nil,
           "null island is still refused, so adopting the core's gate did not drop the one "
           + "this head already had")
    expect(fix([:])?.fix ?? "", "gps", "the fix kind travels for a head that wants to say why")
    expect(GcsFix(nil) == nil, "and no view at all is no fix")
}

checkMyLocationTrustsTheCoresGate()


func checkTilePyramid() {
    let tile = TileAddress(x: 59492, y: 37374, z: 16)

    guard let parent = TilePyramid.parent(of: tile, depth: 1) else {
        return expect(false, "a tile above zoom 0 has a parent")
    }
    expect(parent == TileAddress(x: 29746, y: 18687, z: 15), "the parent halves both axes")
    expect(TilePyramid.parent(of: TileAddress(x: 0, y: 0, z: 0), depth: 1) == nil,
           "zoom 0 has no parent to fall back to")

    guard let crop = TilePyramid.crop(of: tile, within: parent) else {
        return expect(false, "a tile crops within its own parent")
    }
    expect(crop.size == 0.5, "one level up crops half the parent")
    expect(crop.x == 0 || crop.x == 0.5, "the crop lands on a half boundary")

    guard let grandparent = TilePyramid.parent(of: tile, depth: 2),
          let deepCrop = TilePyramid.crop(of: tile, within: grandparent) else {
        return expect(false, "a tile crops within its grandparent")
    }
    expect(deepCrop.size == 0.25, "two levels up crops a quarter of the parent")

    let stranger = TileAddress(x: 5, y: 5, z: 15)
    expect(TilePyramid.crop(of: tile, within: stranger) == nil,
           "a tile that is not descended from a parent has no crop in it")
    expect(TilePyramid.crop(of: parent, within: tile) == nil,
           "a parent does not crop within its own child")

    let children = TilePyramid.children(of: tile)
    expect(children.count == 4, "a tile has four children one level down")
    expect(children.allSatisfy { $0.z == tile.z + 1 }, "every child is one zoom deeper")
    expect(children.allSatisfy { TilePyramid.parent(of: $0, depth: 1) == tile },
           "every child resolves back to the tile it came from")
    expect(Set(children.map(TilePyramid.quadrant(of:)).map { "\($0.column),\($0.row)" }).count == 4,
           "the four children occupy four distinct quadrants")
}

checkTilePyramid()

func checkVehicleReadiness() {
    let listed = VehicleComponentInfo.list([
        ["name": "Radio", "needsAttention": false as NSNumber],
        ["name": "Sensors", "needsAttention": true as NSNumber],
    ])
    expect(listed.map(\.name).joined(separator: ","), "Radio,Sensors",
           "each component the core lists is carried across, in its order")
    expect(listed[1].needsAttention,
           "and the core decides which needs setup, from setupComplete and requiresSetup together")
    expect(VehicleComponentInfo(["needsAttention": true as NSNumber]) == nil,
           "a nameless component is dropped, because the name is the row and the page it opens")
    expect(VehicleComponentInfo.list(nil).isEmpty, "no answer is no components")

    let ready = VehicleReadiness(["connected": true as NSNumber,
                                  "ready": true as NSNumber, "headline": "Ready to fly",
                                  "detail": "Setup complete and all enabled sensors are healthy."])
    expect(ready.ready == true && ready.headline == "Ready to fly",
           "the summary carries the core's verdict rather than recomputing it")

    let faulty = VehicleReadiness(["connected": true as NSNumber, "ready": false as NSNumber,
                                   "headline": "2 sensors reporting a fault",
                                   "detail": "GPS, Pre-Arm Check"])
    expect(faulty.ready == false && faulty.detail == "GPS, Pre-Arm Check",
           "including the sensor faults, which the core now folds in itself, so this summary no "
           + "longer has to be handed the sensor list by a store that may not have loaded")

    expect(VehicleReadiness([:]) == VehicleReadiness(connected: false, ready: nil,
                                                     headline: "", detail: ""),
           "an empty answer gives no verdict and says nothing, rather than claiming readiness -- "
           + "and it no longer claims the vehicle was checked and found wanting either")

    let none = VehicleReadiness(["connected": false as NSNumber,
                                 "headline": "No vehicle connected",
                                 "detail": "Connect a vehicle to check what it needs."])
    expect(none.verdict == nil,
           "with no vehicle there is nothing whose readiness could be in question, so no verdict "
           + "is offered -- the pill read \u{201C}Check\u{201D} beside a sentence saying a vehicle "
           + "must be connected before anything can be checked, an imperative in the shape of a "
           + "button that answers nothing when pressed")
    expect(none.ready == nil,
           "and the absent verdict is the core's own since d40192d83, not one this head "
           + "reconstructs from connected -- ready is three-state now and a bool cannot hold it")
    expect(VehicleReadiness(["connected": true as NSNumber, "ready": false as NSNumber])
               .verdict?.text ?? "", "Check",
           "a connected vehicle reporting no components at all HAS been checked and is not "
           + "ready, which is a different answer from having nothing to check, and the two were "
           + "the same false before")
    expect(ready.verdict?.text ?? "", "Ready",
           "a connected vehicle that is ready still says so")
    expect(faulty.verdict?.text ?? "", "Check",
           "and one with a fault still asks to be looked at, which is the case the pill is for")
    expect(faulty.verdict?.good == false,
           "drawn as a warning rather than as a confirmation")
}
checkVehicleReadiness()

func checkParameterOptions() {
    let mode = Parameter(name: "FLTMODE1", componentId: 1, json: [
        "units": "", "shortDescription": "Flight mode 1",
        "enumIndex": 2, "enumStrings": ["Stabilize", "Acro", "AltHold"],
        "enumValues": [0, 1, 2], "valueString": "2",
    ])
    expect(mode.value, "AltHold", "an enum parameter shows its label")
    expect(mode.options.count == 3, "and keeps every option it can be set to")
    expect(mode.selectedOption?.raw == "2", "the selected option carries the raw value to write")

    let unknown = Parameter(name: "FLTMODE2", componentId: 1, json: [
        "enumIndex": 3, "enumStrings": ["Stabilize", "Acro", "AltHold", "Unknown: 9"],
        "enumValues": [0, 1, 2, 9], "valueString": "9",
    ])
    expect(unknown.value, "9", "a value outside the enum shows the number, not Unknown")
    expect(unknown.options.count == 3, "the synthetic Unknown entry is not offered as a choice")

    let plain = Parameter(name: "WPNAV_SPEED", componentId: 1, json: [
        "units": "cm/s", "valueString": "500", "enumIndex": -1,
    ])
    expect(plain.options.isEmpty, "a numeric parameter offers no options")
    expect(plain.value, "500", "and shows its value")
    expect(plain.group, "WPNAV", "the group is the prefix before the first underscore")

    let mismatched = Parameter(name: "X_Y", componentId: 1, json: [
        "enumStrings": ["A", "B"], "enumValues": [0], "valueString": "0", "enumIndex": 0,
    ])
    expect(mismatched.options.isEmpty, "labels without matching values are not offered")

    expect(Parameter.rawText(NSNumber(value: 2.0)), "2", "a whole enum value writes without a decimal point")
}

checkParameterOptions()

func checkMissionItemRemoval() {
    let home = MissionItem(view: ["index": 0, "sequence": 0, "name": "Mission Start"], selected: -1)
    let waypoint = MissionItem(view: ["index": 1, "sequence": 1, "name": "Waypoint"], selected: -1)
    expect(!home.canRemove, "the mission start is not a waypoint an operator can delete")
    expect(waypoint.canRemove, "a waypoint can be deleted")
}

checkMissionItemRemoval()

func checkLogEntry() {
    func entry(_ overrides: [String: Any]) -> LogEntry? {
        LogEntry(["index": 0 as NSNumber, "id": 3 as NSNumber, "sizeText": "1.5 MB",
                  "status": "Available", "received": true as NSNumber,
                  "timeState": "known",
                  "time": "2026-09-09T05:25:33.000"]
            .merging(overrides) { _, new in new })
    }

    expect(entry([:])?.sizeText ?? "", "1.5 MB", "the size is the core's sentence")
    expect(entry([:])?.id == 3, "and the log keeps the number the vehicle gave it")
    expect(LogEntry(["sizeText": "1 KB"]) == nil, "an entry with no number is not a log")

    expect(entry(["timeState": "unreceived"])?.timeText ?? "?", "",
           "a log the vehicle has not sent yet shows no time at all, as QGC shows none")
    expect(entry(["timeState": "unknown"])?.timeText ?? "", "Date Unknown",
           "a clock that was never set says so rather than claiming a date")
    expect(entry(["timeState": "known", "time": "not a date"])?.timeText ?? "", "not a date",
           "and something unparseable is shown as sent rather than swallowed")
    expect(!(entry([:])?.timeText ?? "").isEmpty,
           "a log with a real clock is formatted for the reader")
    expect(!(entry([:])?.timeText ?? "").contains("T"),
           "in the reader's own locale rather than the wire's ISO spelling")

    expect(entry(["timeState": "somethingNew"])?.timeState == .unrecognised,
           "a state the core adds later is unrecognised rather than silently read as known")
    expect(entry(["timeState": "somethingNew"])?.timeText ?? "?", "",
           "and shows nothing rather than a date it cannot vouch for")

    var parts = DateComponents()
    parts.year = 2026
    parts.month = 9
    parts.day = 8
    parts.hour = 14
    parts.minute = 42
    parts.second = 51
    let reader = DateFormatter()
    reader.dateStyle = .medium
    reader.timeStyle = .short
    if let instant = Calendar.current.date(from: parts) {
        expect(entry(["timeState": "known", "time": "2026-09-08T14:42:51.000"])?.timeText ?? "",
               reader.string(from: instant),
               "QDateTime::fromSecsSinceEpoch gives a LOCAL-time value and Qt's ISO form omits the "
               + "zone designator for one, so the wire carries wall-clock with no Z. Reading it as "
               + "UTC and rendering it back in the reader's zone shifted every log by the offset - "
               + "14:42 displayed as 18:42 here. This assertion is inert on a machine running UTC, "
               + "where the defect has no effect either")
    } else {
        expect(false, "the reader's calendar can express 2026-09-08 14:42:51")
    }
}

checkLogEntry()

func checkTerrainProfile() {
    func point(_ x: Double, _ mission: Double, _ terrain: Any = 585.0 as NSNumber,
               _ collision: Bool = false) -> [String: Any] {
        ["x": x as NSNumber, "missionAltitude": mission as NSNumber, "terrainAltitude": terrain,
         "collision": collision as NSNumber]
    }
    func profile(_ overrides: [String: Any]) -> TerrainProfile {
        TerrainProfile(["usable": true as NSNumber, "groundKnown": true as NSNumber,
                        "hasCollision": false as NSNumber, "unknownTerrain": 0 as NSNumber,
                        "minAltitudeMeters": 570.0 as NSNumber,
                        "maxAltitudeMeters": 660.0 as NSNumber,
                        "distanceText": "570 m", "lowestText": "570 m", "highestText": "660 m",
                        "points": [point(0, 585), point(0.5, 627, 590.0 as NSNumber),
                                   point(1, 627, 640.0 as NSNumber, true)]]
            .merging(overrides) { _, new in new })
    }

    let drawn = profile([:])
    expect(drawn.usable, "the core says three placed points make a profile")
    expect(drawn.points.count == 3, "and all three decode")
    expect(drawn.distanceText, "570 m", "the span is the core's sentence in the operator's units")
    expect(drawn.lowestText, "570 m", "and so is the floor of the band")
    expect(drawn.highestText, "660 m", "and its ceiling")

    expect(drawn.x(drawn.points[2], width: 100) == 100,
           "the core sends x as a share of the span, so the last point sits at the right edge")
    expect(drawn.x(drawn.points[0], width: 100) == 0, "the first sits at the left")
    let top = drawn.y(drawn.maxAltitude, height: 50)
    let bottom = drawn.y(drawn.minAltitude, height: 50)
    expect(abs(top) < 0.001, "the highest altitude maps to the top of the plot")
    expect(abs(bottom - 50) < 0.001, "the lowest maps to the bottom")
    expect(profile(["minAltitudeMeters": 100.0 as NSNumber,
                    "maxAltitudeMeters": 100.0 as NSNumber]).y(100, height: 50) == 25,
           "a band of no range puts the line down the middle rather than dividing by zero")

    expect(TerrainPoint(point(0.5, 100, NSNull()))?.terrainAltitude == nil,
           "a point over unmapped ground keeps its altitude missing rather than sinking to zero, "
           + "which would draw the ground at sea level")
    expect(TerrainPoint(["x": 0.5 as NSNumber, "terrainAltitude": 90.0 as NSNumber]) == nil,
           "a point the core sent without a mission altitude is dropped, as QGC drops a segment "
           + "whose AMSL altitude is not a number")
    expect(TerrainPoint(point(0.5, 100)) != nil, "while a complete point decodes")

    expect(profile(["unknownTerrain": 1 as NSNumber]).unknownTerrain == 1,
           "the count of points over unmapped ground is the core's")
    expect(!profile(["groundKnown": false as NSNumber]).groundKnown,
           "a partly known ground is not drawn as if it were the ground")
    expect(profile(["hasCollision": true as NSNumber]).hasCollision, "a collision is reported")

    expect(!TerrainProfile.empty.usable, "an empty profile is not drawn")
    expect(TerrainProfile([:]).points.isEmpty, "and a read that returned nothing plots nothing")
}

checkTerrainProfile()

func checkMissionCommands() {
    let commands = MissionCommand.from([
        ["command": 16, "friendlyName": "Waypoint", "category": "Basic"],
        ["command": 21, "friendlyName": "Land", "category": "Basic"],
        ["command": 16, "friendlyName": "Waypoint", "category": "Basic"],
        ["command": 99, "friendlyName": "", "category": "Basic"],
        ["friendlyName": "No command number"],
    ])
    expect(commands.count == 2, "duplicates, nameless and numberless commands are dropped")
    expect(commands.first?.name == "Waypoint", "the friendly name is what an operator picks from")

    let described = MissionCommand.from([
        ["command": 19, "friendlyName": "Loiter (time)", "category": "Loiter",
         "description": "Loiter around the specified position for an amount of time."],
        ["command": 16, "friendlyName": "Waypoint", "category": "Basic"],
    ])
    expect(described[0].summary, "Loiter around the specified position for an amount of time.",
           "the description QGC lists beside the name comes through")
    expect(described[0].category, "Loiter", "and so does the category it was listed under")
    expect(described[1].summary, "",
           "a command with no description reads as empty, not as a missing row")

    let waypoint = MissionItem(view: [
        "index": 1, "sequence": 1, "name": "Waypoint", "simple": true], selected: -1)
    let home = MissionItem(view: [
        "index": 0, "sequence": 0, "name": "Mission Start", "simple": true, "kind": "settings"], selected: -1)
    let survey = MissionItem(view: [
        "index": 2, "sequence": 2, "name": "Survey", "simple": false, "kind": "survey"], selected: -1)
    expect(waypoint.canChangeCommand, "a simple waypoint can become another command")
    expect(!home.canChangeCommand, "mission start is not a command an operator retypes")
    expect(!survey.canChangeCommand, "a complex item is not a simple command swap")
}

checkMissionCommands()

func checkItemFacts() {
    let facts = ItemFact.from([
        ["name": "Delay", "valueString": "45", "units": "secs"],
        ["name": "Mode", "valueString": "Hold", "units": "", "enumStrings": ["Hold", "Continue"]],
        ["valueString": "12"],
    ], list: "textFieldFacts", label: { "<\($0)>" })

    expect(facts.count == 2, "a fact with no name is dropped")
    expect(facts[0].id, "textFieldFacts.0", "a fact is addressed by its list and position")
    expect(facts[1].id, "textFieldFacts.1", "positions follow the order the controller reported")

    let owned = ItemFact.owned([
        ["name": "Grid angle", "property": "gridAngle", "valueString": "0", "units": "deg"],
        ["name": "Refly", "property": "refly90Degrees", "valueString": "false", "readOnly": true],
        ["name": "No property", "valueString": "1"],
    ], label: { "<\($0)>" })
    expect(owned.count == 2, "a fact with no property is not addressable and is dropped")
    expect(owned[0].id, "gridAngle", "an item's own fact is addressed by its property")
    expect(!owned[0].readOnly, "an editable fact is editable")
    expect(owned[1].readOnly, "a read-only fact says so")
    expect(owned[0].title, "<Grid angle>",
           "a fact the vehicle did not describe is named by the core \u{2014} this fixture has no "
           + "shortDescription, and the old assertion only looked right because humanising "
           + "\"Grid angle\" returned it unchanged")

    let identifiers = ItemFact.owned([
        ["name": "TurnAroundDistanceMultiRotor", "property": "turnAroundDistance", "valueString": "10"],
        ["name": "HoverAndCapture", "property": "hoverAndCapture", "valueString": "false", "typeIsBool": true],
    ], label: { "<\($0)>" })
    let served: [Any] = [
        ["name": "SensorWidth", "property": "sensorWidth", "valueString": "7.6", "units": "mm"],
        ["name": "FocalLength", "property": "focalLength", "valueString": "22", "units": "mm"],
        ["name": "FrontalOverlap", "property": "frontalOverlap", "valueString": "70", "units": "%"],
        ["name": "DistanceToSurface", "property": "distanceToSurface", "valueString": "50", "units": "m"],
        ["name": "SideOverlap", "property": "sideOverlap", "valueString": "70", "units": "%"],
        ["name": "ImageDensity", "property": "imageDensity", "valueString": "1.8", "units": "cm/px"],
    ]
    let cameraFacts = ItemFact.camera(served, custom: false, label: { "<\($0)>" })
    expect(cameraFacts.count == 4, "only the four that decide a survey are offered")
    expect(cameraFacts[0].id, "cameraCalc.distanceToSurface", "a camera fact is addressed through cameraCalc")
    expect(cameraFacts[0].group, ItemFact.cameraGroup, "and is grouped as camera")
    expect(!cameraFacts.contains { $0.name == "SensorWidth" }, "camera hardware specs are not survey settings")
    expect(cameraFacts.map(\.name).joined(separator: ","), ["DistanceToSurface", "ImageDensity", "FrontalOverlap", "SideOverlap"].joined(separator: ","),
           "they read in the order an operator thinks about them")

    let custom = ItemFact.camera(served, custom: true, label: { "<\($0)>" })
    expect(custom.contains { $0.name == "SensorWidth" },
           "but Custom Camera is in the brand list this head draws -- second of twelve -- and "
           + "choosing it IS specifying the camera rather than picking one that is already "
           + "specified. Measured on the running app: the operator could select it and then had "
           + "no field for sensor size, image size or focal length anywhere, so the footprint "
           + "and the shot count computed from whatever the last catalogue camera left behind")
    expect(custom.contains { $0.name == "FocalLength" },
           "the optics are the whole definition of a custom camera, so all of them are offered "
           + "rather than a couple: sensor width and height, image width and height, focal "
           + "length, orientation and the minimum trigger interval")
    expect(custom.count == 6,
           "and the four that decide the survey are still there -- the optics are ADDED for a "
           + "custom camera, not swapped in. Two optics appear in this fixture, so six")
    expect(custom.first?.name == "SensorWidth",
           "the optics come first: an operator specifies the camera before tuning how the "
           + "survey uses it, and reversing that asks them to tune against a camera they have "
           + "not described yet")
    expect(cameraFacts.count == 4,
           "and a CATALOGUE camera is unchanged. The old rule stays as its own control: picking "
           + "a Canon supplies the optics, so offering them there would be an editable copy of "
           + "something the catalogue already answered")

    expect(identifiers[0].title != "TurnAroundDistanceMultiRotor",
           "an undescribed fact is not shown as a raw identifier")
    expect(identifiers[1].isBool, "a boolean fact is a toggle, not a text field")
    expect(!identifiers[0].isBool, "a numeric fact is not")
    expect(facts[0].options.isEmpty, "a plain fact offers no options")
    expect(facts[1].options.count == 2, "an enum fact offers its choices")
    expect(facts[0].units, "secs", "units come through for the editor to show")
}

checkItemFacts()

func checkUnplacedCommands() {
    let delay = MissionItem(view: ["index": 1, "sequence": 1, "name": "Delay", "simple": true], selected: -1)
    expect(!delay.hasPosition, "a command the core gave no place is not placed")
    expect(delay.positionText, "\u{2014}", "and shows nothing rather than the Gulf of Guinea")
    expect(!delay.flownLeg, "and no leg is drawn to it")

    let waypoint = MissionItem(view: [
        "index": 2, "sequence": 2, "name": "Waypoint", "flownLeg": true,
        "coordinate": ["latitude": -35.363, "longitude": 149.165]], selected: -1)
    expect(waypoint.hasPosition, "a waypoint the core placed is placed")
    expect(waypoint.flownLeg, "and the vehicle flies a leg to it")

    let roi = MissionItem(view: [
        "index": 3, "sequence": 3, "name": "Region of interest", "flownLeg": false,
        "coordinate": ["latitude": -35.33, "longitude": 149.25]], selected: -1)
    expect(roi.hasPosition,
           "a region of interest has a place and earns a marker -- measured on the running app, "
           + "where the core listed it with a coordinate and flownLeg false")
    expect(!roi.flownLeg,
           "and the vehicle never goes there. One list answered both questions, so the planned "
           + "route drew a leg out to the ROI and back: a dogleg to a point the aircraft does not "
           + "visit, on the map an operator checks the shape of the mission against")
}

checkUnplacedCommands()

func checkMapWindow() {
    let centre = GeoPoint(latitude: -35.36, longitude: 149.16)
    guard let window = MapWindow(centre: centre, latitudeSpan: 0.02, longitudeSpan: 0.04) else {
        expect(false, "a map with a span makes a window")
        return
    }
    expect(abs(window.topLeft.latitude - (-35.35)) < 1e-9,
           "the top-left corner is north of centre")
    expect(abs(window.topLeft.longitude - 149.14) < 1e-9,
           "and west of it")
    expect(abs(window.bottomRight.latitude - (-35.37)) < 1e-9,
           "the bottom-right corner is south")
    expect(abs(window.bottomRight.longitude - 149.18) < 1e-9,
           "and east")

    expect(MapWindow(centre: centre, latitudeSpan: 0, longitudeSpan: 0.04) == nil,
           "a map with no height gives no window to put a fence in")
    expect(MapWindow(centre: nil, latitudeSpan: 0.02, longitudeSpan: 0.04) == nil,
           "and neither does one that has not settled on a centre")
}

checkMapWindow()

func checkGuidedRange() {
    let altitude: [String: Any] = ["available": true as NSNumber, "unit": "m",
                                   "label": "Height above launch",
                                   "minimum": 5.0 as NSNumber, "maximum": 121.0 as NSNumber,
                                   "current": 40.0 as NSNumber]
    guard let above = GuidedRange(altitude) else {
        return expect(false, "the core's altitude range is usable")
    }
    expect(above.initial == 40, "the slider starts where the aircraft already is")
    expect(above.label, "Height above launch", "and says what the height is measured from")
    expect(above.text(40), "40 m", "the core has already converted, so the head only appends its unit")

    let feet: [String: Any] = ["available": true as NSNumber, "unit": "ft",
                               "minimum": 16.4 as NSNumber, "maximum": 397.0 as NSNumber,
                               "current": 131.2 as NSNumber]
    expect(GuidedRange(feet)?.text(131.2) ?? "", "131 ft",
           "in feet the head appends the core's unit rather than converting anything itself")

    let speed: [String: Any] = ["available": true as NSNumber, "unit": "m/s", "label": "Airspeed",
                                "minimum": 8.0 as NSNumber, "maximum": 20.0 as NSNumber,
                                "initial": 14.0 as NSNumber]
    let air = GuidedRange(speed)
    expect(air?.label ?? "", "Airspeed", "a view that carries its own label keeps it")
    expect(air?.text(14) ?? "", "14 m/s", "a speed the operator can read at a glance")
    expect(air?.text(0.4) ?? "", "8.0 m/s",
           "a value under ten keeps a decimal, so the slowest ground speed cannot render as a stationary 0 m/s")

    expect(GuidedRange(["available": false as NSNumber, "minimum": 5.0 as NSNumber,
                        "maximum": 121.0 as NSNumber]) == nil,
           "a view that says it is unavailable offers no slider, whatever else it carries")
    expect(GuidedRange(["available": true as NSNumber, "minimum": 10.0 as NSNumber,
                        "maximum": 10.0 as NSNumber]) == nil,
           "and an empty range is not a slider")
    expect(GuidedRange(nil) == nil, "no answer is no range")

    let clipped = GuidedRange(["available": true as NSNumber, "unit": "m",
                               "minimum": 5.0 as NSNumber, "maximum": 121.0 as NSNumber,
                               "current": 400.0 as NSNumber])
    expect(clipped?.initial == 121, "a start outside the range is pulled into it")
    expect(clipped?.clamped(900) == 121, "as is anything the slider is asked for")
}

checkGuidedRange()

func checkFlyDetail() {
    let battery = [
        FactReading(name: "voltage", value: "12.60", units: "v"),
        FactReading(name: "current", value: "0.00", units: "A"),
        FactReading(name: "mahConsumed", value: "0", units: "mAh"),
        FactReading(name: "temperature", value: "--.--", units: "C"),
        FactReading(name: "timeRemainingStr", value: "--:--:--", units: ""),
        FactReading(name: "instantPower", value: "0.00", units: "W"),
        FactReading(name: "chargeState", value: "1", units: ""),
    ]
    let pack = FlyDetail.battery(battery)
    expect(pack.map(\.label).joined(separator: ","), "Voltage,Current,Power,Consumed",
           "a temperature and a time the pack never reported are left out, not shown as dashes")
    expect(pack[0].value, "12.60 V",
           "a reading carries its units, with the volt capitalised as the summary row has it")

    expect(FlyDetail.battery([]).isEmpty, "a vehicle with no battery facts has no detail to open")

    let gps = FlyDetail.gps([
        FactReading(name: "lat", value: "-35.3632621", units: ""),
        FactReading(name: "lon", value: "149.1652374", units: ""),
        FactReading(name: "hdop", value: "1.2", units: ""),
        FactReading(name: "count", value: "10", units: ""),
        FactReading(name: "courseOverGround", value: "0.0", units: "deg"),
    ])
    expect(gps.first?.label ?? "", "Position", "position leads, because it is what an operator looks for")
    expect(gps.first?.value ?? "", "-35.3632621, 149.1652374", "and reads as one pair")
    expect(gps.map(\.label).joined(separator: ","),
           "Position,Satellites,HDOP,Course over ground",
           "the rest follow in the order QGC lists them, minus what was not reported")

    expect(FlyDetail.gps([FactReading(name: "lat", value: "1.0", units: "")]).isEmpty,
           "half a position is no position")

    expect(FlyDetail.link(rcRSSI: 255, localRSSI: 0, remoteRSSI: 0).isEmpty,
           "a TCP link reports no radio at all, so the link row stays away")
    expect(FlyDetail.link(rcRSSI: 84, localRSSI: -70, remoteRSSI: -68).map(\.label)
        .joined(separator: ","), "RC signal,Telemetry here,Telemetry on the vehicle",
           "a real radio reports all three")
    expect(FlyDetail.link(rcRSSI: 0, localRSSI: nil, remoteRSSI: nil).map(\.value)
        .joined(separator: ","), "No signal",
           "a zero RC reading is a dead stick, not absence: Vehicle stores 255 when it has "
           + "nothing to say and an explicit 0 once the filtered signal decays, so the row that "
           + "used to vanish now says so - a row that disappears reads as not applicable")
    expect(FlyDetail.rcSignal(255) == nil,
           "255 is the vehicle having nothing to report, and still shows no row")
    expect(FlyDetail.rcSignal(0) ?? "", "No signal", "0 is the vehicle reporting silence")
    expect(FlyDetail.rcSignal(84) ?? "", "84%", "and a live reading is a plain percentage")
    expect(FlyDetail.rcSignal(100) ?? "", "100%", "including the top of the range")
    expect(FlyDetail.rcSignal(101) == nil,
           "a percentage QGC would not accept is dropped rather than rendered; the old guard "
           + "only excluded 255 and would have printed 150%")
    expect(FlyDetail.rcSignal(254) == nil, "and so is anything else short of the sentinel")

    expect(Units.display("v"), "V",
           "the vehicle spells volts in lower case; the whole window spells it the same way")
    expect(Units.display("m/s"), "m/s", "everything else is left as the vehicle sent it")
}

checkFlyDetail()

func checkItemSpeed() {
    let off = ItemSpeed(json: ["available": true, "specifyFlightSpeed": false,
                               "facts": [["name": "FlightSpeed", "value": 8.0, "units": "m/s"]]])
    expect(off.available && !off.specified, "an item that can set a speed but does not")
    expect(off.note, "This item flies at whatever speed the one before it set.",
           "and says where its speed comes from instead, which is what QGC's unchecked box means")

    let on = ItemSpeed(json: ["available": true, "specifyFlightSpeed": true,
                              "facts": [["name": "FlightSpeed", "value": 4.5, "units": "ft/s"]]])
    expect(on.specified && on.value == 4.5, "one that does carries its own value")
    expect(on.units, "ft/s", "in the units the fact came with, not a hard-coded m/s")
    expect(on.note, "This item flies at its own speed.", "and says so")

    expect(ItemSpeed.factName, "FlightSpeed",
           "the fact carries the name SpeedSection.cc:16 gives it, and that is the string the "
           + "facts list is searched for")
    expect(ItemSpeed.property, "flightSpeed",
           "while the property is the one SpeedSection.h:26 declares, and it is interpolated into "
           + "a WRITE path in Mission.swift. Swapping the two satisfies an inequality check and "
           + "then finds no fact and writes to a property that does not exist, both in silence")

    expect(!ItemSpeed(json: [:]).available,
           "an item with no speed section offers nothing rather than a dead control")
    expect(ItemSpeed(json: ["available": true, "specifyFlightSpeed": true]).value == nil,
           "and one whose speed fact is missing shows no number rather than a zero")
}

checkItemSpeed()
checkMissionStartSpeed()
checkLaunchPosition()
checkLaunchAltitudeIsNotListedTwice()



func checkVehicleSetupText() {
    expect(VehicleSetupText.waiting(connected: true, for: "parameters"),
           "Reading parameters from the vehicle\u{2026}",
           "with a vehicle attached, the page really is waiting on it")
    expect(VehicleSetupText.waiting(connected: false, for: "parameters"),
           "Connect a vehicle to see its parameters.",
           "with none, saying it is reading claims a transfer that is not happening")

    expect(VehicleSetupText.absent(connected: true, "is not reporting a battery."),
           "This vehicle is not reporting a battery.",
           "a connected vehicle that omits something is described as omitting it")
    expect(VehicleSetupText.absent(connected: false, "is not reporting a battery."),
           "Connect a vehicle to set this up.",
           "but with no vehicle there is no this vehicle to report anything")

    expect(VehicleSetupText.filtered(connected: true), "No parameter matches this filter.",
           "an empty list under a filter is the filter's doing")
    expect(VehicleSetupText.filtered(connected: false),
           "Connect a vehicle to see its parameters.",
           "an empty list with no vehicle is not")
}

checkVehicleSetupText()

func checkCalibrationOrder() {
    func routine(_ id: String, _ blocked: Bool, _ enabled: Bool,
                 _ description: String) -> [String: Any] {
        ["id": id, "title": id, "invocation": "sensorsCal.\(id)", "arguments": [],
         "blocked": blocked as NSNumber, "enabled": enabled as NSNumber,
         "description": description, "warning": ""]
    }
    let needsAccel = CalibrationState(["connected": true as NSNumber, "routines": [
        routine("accelerometer", false, true, "Hold the vehicle in each orientation it asks for."),
        routine("compass", true, false, "Calibrate the accelerometer first."),
        routine("levelHorizon", true, false, "Calibrate the accelerometer first."),
        routine("gyro", false, true, "Leave the vehicle still while the gyros settle."),
        routine("pressure", false, true, "Zero the barometer at the current altitude."),
    ]])
    let by = { (id: String) in needsAccel.routines.first { $0.id == id } }

    expect(needsAccel.routines.count == 5, "five routines are offered")
    expect(by("compass")?.blocked ?? false,
           "a compass calibration on an uncalibrated accelerometer gives a result to distrust")
    expect(by("levelHorizon")?.blocked ?? false,
           "and so does levelling the horizon against one")
    expect(!(by("accelerometer")?.blocked ?? true),
           "the accelerometer itself is the way out, so it is never blocked")
    expect(!(by("gyro")?.blocked ?? true), "the gyro does not depend on it")
    expect(!(by("pressure")?.blocked ?? true), "nor does the barometer")
    expect(by("compass")?.description ?? "", "Calibrate the accelerometer first.",
           "a blocked routine says why rather than only greying out")
    expect(!(by("compass")?.enabled ?? true),
           "and cannot be started, which is the only gate the head has left")
}

checkCalibrationOrder()

func checkFlightModes() {
    func mode(_ name: String, _ advanced: Bool = false, _ current: Bool = false,
              _ needsConfirm: Bool = false, _ summary: String = "") -> [String: Any] {
        ["name": name, "summary": summary, "advanced": advanced as NSNumber,
         "current": current as NSNumber, "needsConfirm": needsConfirm as NSNumber]
    }

    let choices = FlightModes.list([
        mode("Stabilize"), mode("Altitude Hold"), mode("Auto"),
        mode("Guided", false, true), mode("Loiter"),
        mode("RTL", false, false, true, "Climbs, returns home and lands"),
        mode("Land", false, false, true), mode("Position Hold"),
        mode("Acro", true), mode("Circle", true), mode("Turtle", true),
    ])
    expect(choices.count == 11, "every mode the core listed is offered")
    expect(choices.first { $0.current }?.name ?? "", "Guided", "and the one it is in is marked")
    expect(FlightModes.everyday(choices).map(\.name).joined(separator: ","),
           "Stabilize,Altitude Hold,Auto,Guided,Loiter,RTL,Land,Position Hold",
           "the everyday list is what is left once the advanced modes are folded away")
    expect(FlightModes.folded(choices).map(\.name).joined(separator: ","), "Acro,Circle,Turtle",
           "and the folded list is the rest")

    let inAdvanced = FlightModes.list([mode("Loiter"), mode("Circle", true, true),
                                       mode("Turtle", true)])
    expect(FlightModes.everyday(inAdvanced).map(\.name).contains("Circle"),
           "a vehicle already in an advanced mode still shows it without opening More modes")
    expect(!FlightModes.folded(inAdvanced).map(\.name).contains("Circle"),
           "and it is not listed twice")

    expect(choices.first { $0.name == "RTL" }?.summary ?? "", "Climbs, returns home and lands",
           "each mode carries the sentence the core writes for it")
    expect(choices.first { $0.name == "Stabilize" }?.summary ?? "", "",
           "and a mode the core has no sentence for gets none rather than a wrong one")

    expect(choices.first { $0.name == "RTL" }?.needsConfirm ?? false,
           "sending a flying vehicle home is confirmed, and the core decides that now, from the "
           + "vehicle's own rtlFlightMode and landFlightMode rather than two strings kept here")
    expect(choices.first { $0.name == "Land" }?.needsConfirm ?? false, "so is landing it")
    expect(!(choices.first { $0.name == "Loiter" }?.needsConfirm ?? true),
           "holding position is not a commitment and needs no second tap")

    expect(FlightModes.symbol(for: "Smart RTL"), "house", "the glyph follows the mode's meaning")
    expect(FlightModes.symbol(for: "QuadPlane Land"), "arrow.down.to.line",
           "including a firmware-specific spelling of it")
    expect(FlightModes.symbol(for: "Mode 65536"), "airplane", "and an unknown mode still gets one")

    expect(FlightModes.list([]).isEmpty,
           "a vehicle that has not reported its modes offers none, rather than a stale list")
    expect(FlightModes.list([mode("")]).isEmpty, "and a nameless mode is dropped")
}

checkFlightModes()

func checkFenceUsable() {
    expect(FenceShape(["index": 0 as NSNumber, "shape": "polygon",
                       "usable": true as NSNumber])?.usable == true,
           "the core decides whether a fence has enough geometry to draw")
    expect(FenceShape(["index": 0 as NSNumber, "shape": "circle",
                       "usable": false as NSNumber])?.usable == false,
           "a circle with no centre is not usable, and the head no longer works that out")
}

checkFenceUsable()

func checkRallyAndBreach() {
    let listed = RallyPointRow.list([
        ["index": 0 as NSNumber, "path": "plan.rallyPointController.points.0",
         "latitude": -35.3628 as NSNumber, "longitude": 149.1665 as NSNumber,
         "altitude": 30.0 as NSNumber, "altitudeUnits": "m", "altitudeText": "30.0 m",
         "altitudePath": "plan.rallyPointController.points.0.textFieldFacts.2"],
        ["index": 1 as NSNumber, "path": "plan.rallyPointController.points.1",
         "latitude": -35.36 as NSNumber, "longitude": 149.16 as NSNumber,
         "altitude": 164.0 as NSNumber, "altitudeUnits": "ft", "altitudeText": "164 ft",
         "altitudePath": "plan.rallyPointController.points.1.textFieldFacts.2"],
    ])
    expect(listed.count == 2, "each rally point the core lists is carried across")
    expect(listed[0].positionText, "-35.362800, 149.166500", "with its position to six places")
    expect(listed[0].altitudeText, "30.0 m",
           "and its height is the string the CORE spelled -- cf5ab9407 added altitudeText to a "
           + "rally point, so this head no longer runs the number back through its own copy of "
           + "format_measure. That copy was missing settled() until this evening and drew "
           + "\"-0.0 m\" for a station at field level")
    expect(listed[1].altitudeText, "164 ft",
           "so a station in feet reads in feet rather than being converted twice")
    expect(listed[0].altitude == 30.0,
           "and the RAW number and unit stay, because a rally altitude is TYPED: the row hands "
           + "them to an editor through altitudePath. The core owns what is read; the head needs "
           + "the number for what is edited")
    expect(RallyPointRow.list([["index": 0 as NSNumber]]).first?.altitudeText ?? "",
           Measure.unreported,
           "a point the core spelled no altitude for reads as absent rather than as a zero")
    expect(listed[1].altitudePath, "plan.rallyPointController.points.1.textFieldFacts.2",
           "the core names the fact to write, so the head no longer hunts the textFieldFacts "
           + "array for one called RelativeAltitude")

    expect(RallyPointRow(["path": "p"]) == nil, "a point with no index is dropped")
    expect(RallyPointRow.list(nil).isEmpty, "no answer is no rally points")

    let noHeight = RallyPointRow(["index": 0 as NSNumber, "latitude": -35.36 as NSNumber,
                                  "longitude": 149.16 as NSNumber])
    expect(noHeight?.altitudeText ?? "", "\u{2014}",
           "a point whose altitude fact the core could not read shows a dash, not a zero height")

    let breach = RallyPointRow(coordinate: ["latitude": -35.36 as NSNumber,
                                            "longitude": 149.16 as NSNumber])
    expect(breach.positionText, "-35.360000, 149.160000",
           "the breach return point is still built from a bare coordinate, because it is not "
           + "a rally point and the core does not list it")
    expect(breach.altitudeText, "\u{2014}",
           "and carries no height of its own; the fence's breach altitude is a separate fact")

    let nowhere = RallyPointRow(coordinate: nil)
    expect(nowhere.positionText, "\u{2014}", "with no coordinate it claims no position")
}

checkRallyAndBreach()

func checkMissionItemKinds() {
    func kind(_ id: String, _ title: String, _ invokable: String, _ complex: Any = NSNull(),
              _ geometry: Any = NSNull(), _ property: Any = NSNull(), _ noun: String = "shape",
              _ hint: String = "", _ refusal: Any = NSNull()) -> [String: Any] {
        ["id": id, "title": title, "invokable": invokable, "complexName": complex,
         "geometry": geometry, "geometryProperty": property, "shapeNoun": noun,
         "placementHint": hint, "simple": (complex is NSNull) as NSNumber,
         "enabled": (refusal is NSNull) as NSNumber, "disabledReason": refusal]
    }
    let catalogue = MissionKinds(["kinds": [
        kind("waypoint", "Waypoint", "insertSimpleMissionItem", NSNull(), NSNull(), NSNull(),
             "shape", "Click the map to place a waypoint."),
        kind("takeoff", "Takeoff", "insertTakeoffItem"),
        kind("land", "Land", "insertLandItem"),
        kind("roi", "Region of Interest", "insertROIMissionItem", NSNull(), NSNull(), NSNull(),
             "shape", "Click the map to place a region of interest."),
        kind("survey", "Survey", "insertComplexMissionItem", "Survey", "area",
             "surveyAreaPolygon", "area", "Click the map to place a survey area."),
        kind("corridor", "Corridor Scan", "insertComplexMissionItem", "Corridor Scan", "line",
             "corridorPolyline", "path", "Click the map to place a corridor to scan along."),
        kind("structure", "Structure Scan", "insertComplexMissionItem", "Structure Scan", "area",
             "structurePolygon", "area", "Click the map to place a structure to scan around."),
    ]])

    expect(catalogue.all.count == 7,
           "the add menu offers every item type the core catalogues")
    expect(catalogue.simple.map(\.id).joined(separator: ","), "waypoint,takeoff,land,roi",
           "the simple items are the ones the head inserts by their own call")
    expect(catalogue.shapeImportable.map(\.id).joined(separator: ","), "survey,corridor,structure",
           "only the three complex patterns can be drawn from a shape file")
    expect(catalogue.byId("survey")?.invokable ?? "", "insertComplexMissionItem",
           "and each carries the controller call that inserts it")
    expect(catalogue.byId("helix") == nil, "an unknown kind is not invented")
    expect(catalogue.byId("") == nil,
           "an empty id is no kind, which is how the Empty template asks for nothing")

    expect(catalogue.byComplexName("Corridor Scan")?.id ?? "", "corridor",
           "a pattern the core knows is matched by the name the controller uses")

    expect(catalogue.byComplexName("Fixed Wing Landing Pattern") == nil,
           "a Landing Pattern is a real QGC complex item the catalogue does not name, so it is "
           + "unknown rather than mistaken for another")
    expect(catalogue.title(forPattern: "Fixed Wing Landing Pattern"), "Fixed Wing Landing Pattern",
           "an unknown one is offered under the name the vehicle gave it, not hidden")
    expect(catalogue.symbol(forPattern: "VTOL Landing Pattern"), "square.on.square.dashed",
           "and still gets a glyph")
    expect(catalogue.placementHint(forPattern: "Fixed Wing Landing Pattern"),
           "Click the map to place a fixed wing landing pattern.",
           "with a hint that names it")
    expect(catalogue.title(forPattern: "Survey"), "Survey", "a known pattern keeps its own title")
    expect(catalogue.symbol(forPattern: "Survey"), "square.grid.3x3",
           "and the glyph this head draws for it, which is the head's own and not the core's")

    expect(MissionKinds.empty.title(forPattern: "Survey"), "Survey",
           "before the catalogue has loaded a pattern still names itself rather than vanishing")

    let refusing = MissionKinds(["kinds": [
        kind("waypoint", "Waypoint", "insertSimpleMissionItem"),
        kind("takeoff", "Takeoff", "insertTakeoffItem", NSNull(), NSNull(), NSNull(), "shape", "",
             "This mission already takes off."),
        kind("survey", "Survey", "insertComplexMissionItem", "Survey", "area",
             "surveyAreaPolygon", "area", "", "Nothing can follow the landing."),
    ]])
    expect(refusing.byId("takeoff")?.enabled == false,
           "the core says which kinds can go in this mission now, and the head no longer offers "
           + "a takeoff to a mission that already has one")
    expect(InsertOutcome(["ok": true as NSNumber, "inserted": "survey"]) == .inserted,
           "the core inserting the item is the whole answer, and the head reads it rather than "
           + "deciding from its own polled catalogue, which can be a poll behind")
    expect(InsertOutcome(["ok": false as NSNumber, "unknown": "Fixed Wing Landing Pattern",
                          "reason": "the core has no Fixed Wing Landing Pattern in its catalogue"])
        == .insertDirectly("Fixed Wing Landing Pattern"),
           "a name the core never listed is not one it refused -- QGC creates these -- and its "
           + "lookup fails before it touches the controller, so nothing was inserted and nothing "
           + "needs undoing before the head puts the item in the way it always did")
    expect(InsertOutcome(["ok": false as NSNumber, "refused": "takeoff",
                          "reason": "The mission already takes off before this point."])
        == .refused("The mission already takes off before this point."),
           "while a kind the core knows and turns down carries the core's own sentence")
    expect(InsertOutcome(["ok": false as NSNumber]) == .refused(MissionKinds.refusedWithoutReason),
           "and a refusal with no words still refuses, rather than falling through to the direct "
           + "insert that unknown gets -- silence is not permission to bypass the gate")

    expect(catalogue.byId("takeoff")?.enabled == true,
           "a catalogue that refused nothing enables everything")
    expect(MissionKinds(["kinds": [["id": "takeoff", "title": "Takeoff"]]])
        .byId("takeoff")?.enabled == false,
           "but a kind arriving with no enabled key is NOT enabled: availability is derived from "
           + "the core saying yes, never from it failing to say no. The required-keys row for "
           + "view.missionKinds is what keeps that unreachable, the same way it holds clipping")

    expect(refusing.offers(pattern: "Survey") == false, "a refused pattern is not offered")
    expect(refusing.offers(pattern: "Fixed Wing Landing Pattern"),
           "while one the catalogue does not name stays offered, because hiding it would drop a "
           + "real QGC item the core has no opinion about")

    let seed = MissionSeed(["property": "surveyAreaPolygon", "points": [
        ["latitude": -35.3644 as NSNumber, "longitude": 149.1636 as NSNumber],
        ["latitude": -35.3644 as NSNumber, "longitude": 149.1663 as NSNumber],
        ["latitude": -35.3617 as NSNumber, "longitude": 149.1663 as NSNumber],
        ["latitude": -35.3617 as NSNumber, "longitude": 149.1636 as NSNumber],
    ]])
    expect(seed?.points.count == 4, "a new survey gets a four cornered area rather than an empty one")
    expect(seed?.property ?? "", "surveyAreaPolygon", "appended to the property the core named")
    expect(MissionSeed(["property": "surveyAreaPolygon", "points": []]) == nil,
           "a seed with no points is no seed, rather than an empty shape on the map")
    expect(MissionSeed([:]) == nil, "and a kind with no geometry seeds nothing at all")

    checkHostNotices()
    checkBlockedItems()
    checkBridgeWatchers()
    checkRemoveOutcome()
    checkSurveyWatch()
    checkCollisionRuns()
    checkClearanceSentence()
    checkUnreachedItems()
    checkLegsSpelled()
    checkWithheldLegFigures()
    checkTerrainMarkers()
    checkOmittedModesSayWhy()
    checkCamerasTheOperatorTurnedOn()
    checkRefusedModesSayWhy()
    checkAltitudeReading()
    checkAltitudeModePickerIsOffered()
    checkGroundDrawnFromKnownSamples()
    checkUnknownMissionTime()
    checkOnlyAPlacedItemMoves()
    checkComplexGeometryInAnyLocale()
    checkSpeedChangeIsListedNotOnlySelected()
    checkAnItemSaysEverythingItDoes()
    checkARouteLeavesAPatternWhereItEnds()
    checkAnItemSaysHowManyCommandsItFolds()
    checkAPatternSaysHowHighAboveTheGroundItFlies()
    checkAPatternDrawsThePathItActuallyFlies()
    checkAnUnreadStoreDoesNotBlameTheVehicle()
    checkSaveAsksTheCoreRatherThanTheReadiness()
    checkAnAltitudeSaysWhatItIsMeasuredFrom()
    checkSensorsComponentIsFoundByClass()
    checkShapeAbsence()
    checkBatteryReading()
    checkFlownLeg()
    checkVehicleMessageOrder()
    checkBlockedBanner()
    checkMavlinkConsole()
    checkModeSlots()
    checkVideoFrame()

    expect(WriteReport.failure("the fence radius"),
           "Could not change the fence radius. It is unchanged.",
           "a refused write names what did not change and says the old value still stands")
    expect(WriteReport.failure("this item's altitude").contains("unchanged"),
           "because the control snapping back on the next poll reads as the app glitching")
}

checkMissionItemKinds()

func checkFlyTelemetry() {
    var reading = FlyTelemetry()
    expect(reading.batteryLevel == .unknown, "no battery reading is unknown, not good")
    expect(reading.gpsLevel == .unknown, "no gps reading is unknown, not good")
    expect(reading.batteryText, "\u{2014}", "and shows nothing rather than a number")

    expect(FlyTelemetry.batteryLine("100%", "12.60V"), "100% \u{00B7} 12.60V",
           "the chip joins the core's two indicator lines")
    expect(FlyTelemetry.batteryLine("12.60V", "12.60V"), "12.60V",
           "and does not say the same thing twice when a pack reports no percentage")
    expect(FlyTelemetry.batteryLine("n/a", ""), "n/a",
           "a pack that reports nothing usable still says so in QGC's own words")
    expect(FlyTelemetry.batteryLine("", ""), "\u{2014}", "and an absent battery shows a dash")

    expect(FlyTelemetry.Level("normal") == .good, "the core's normal is a good battery")
    expect(FlyTelemetry.Level("caution") == .caution, "its caution is yellow, not orange")
    expect(FlyTelemetry.Level("warning") == .warning,
           "its warning is the orange QGC paints for a vehicle-reported LOW, and reading that as "
           + "unknown would have drawn a grey dot for a battery the vehicle called low")
    expect(FlyTelemetry.Level("critical") == .critical, "and its critical is critical")
    expect(FlyTelemetry.Level(nil) == .unknown,
           "no answer is unknown, never good, because the head no longer decides this from a "
           + "percentage and must not invent a level when the core has not given one")
    expect(FlyTelemetry.Level("nonsense") == .unknown,
           "and a level this head does not recognise is unknown rather than the safest-looking one")

    reading.gpsLock = 6
    reading.satellites = 10
    expect(reading.gpsLevel == .good, "an RTK fix is good")
    expect(reading.gpsText, "RTK fixed \u{00B7} 10 sats", "and names the fix and the count")
    reading.gpsLock = 2
    expect(reading.gpsLevel == .warning, "a 2D fix is not enough to trust a position")
    reading.gpsLock = 0
    expect(reading.gpsLevel == .critical, "no fix is critical")
    expect(reading.gpsText, "No fix \u{00B7} 10 sats", "and says so plainly")

}

checkFlyTelemetry()

func checkFlyState() {
    func read(_ token: String, _ line: String, _ extra: [String: Any] = [:]) -> FlyState {
        var json: [String: Any] = ["state": token, "stateText": line]
        extra.forEach { json[$0.key] = $0.value }
        return FlyState(json)
    }

    expect(FlyState.none.display, FlyState.noVehicle,
           "before the first read the line invites a connection rather than naming a state")

    let disarmed = read("disarmed", "Disarmed")
    expect(disarmed.display, "Disarmed", "the core names the state and this head prints its words")
    expect(!disarmed.alarming, "a vehicle sitting disarmed is not an alarm")

    let flying = read("flying", "Flying", ["armed": true as NSNumber, "connected": true as NSNumber])
    expect(flying.display, "Flying", "and an airborne one reads as flying")
    expect(flying.armed, "the badge is drawn from the same reply as the line")

    let lost = read("contactLost", "Communication lost",
                    ["contactLost": true as NSNumber, "armed": true as NSNumber,
                     "staleNotice": "No contact." as Any])
    expect(lost.alarming, "lost contact is the state that colours the line, and it used to be "
           + "read from a raw property this head reached for itself")
    expect(lost.staleNotice, "No contact.",
           "the notice is the core's sentence verbatim; both heads used to hand-write it, so a "
           + "copy edit in one place left the other saying something else")

    expect(read("notConnected", "Not connected").display, FlyState.noVehicle,
           "the core says Not connected; this window says what to do about it, because what an "
           + "empty Fly view should offer is the head's business and not the core's")

    let invented = read("emergency", "Emergency")
    expect(invented.kind == .unknown,
           "a state token this head has never heard of decodes to unknown, not to the first case")
    expect(invented.display, "Emergency",
           "and still prints the core's words, so a state added there is legible here at once")
    expect(invented.alarming,
           "an unrecognised vehicle state draws the eye rather than passing as ordinary; a "
           + "fallback onto a known case would have shown a new emergency in the quiet colour")

    expect(read("emergency", "").display, FlyState.unnamedState,
           "and a token with no words at all still says something, rather than leaving the "
           + "status line blank")
    expect(!read("emergency", "Emergency").armed,
           "an unknown state claims nothing about arming that the reply did not say")

    expect(lost.linkLevel == .critical,
           "the Link chip is coloured by the link, and lost contact is the link failing")
    expect(flying.linkLevel == .good,
           "a vehicle in contact has a healthy link even when it is doing something else")
    let noFix = read("flying", "Flying", ["connected": true as NSNumber])
    expect(noFix.linkLevel == .good,
           "the Link chip took its colour from gpsLevel, so a vehicle with a solid radio and no "
           + "GPS fix reported the radio as critical -- and worse, a good fix painted a failing "
           + "link green, which is the direction that hides a real problem")
}

checkFlyState()

func checkPlanDirtyBadge() {
    expect(PlanDirtyBadge.text(connected: true), PlanDirtyBadge.unsent,
           "connected, sendToVehicle is the only thing that clears dirty -- "
           + "PlanMasterController.cc:283 and :330 -- so the flag is upload-pending")
    expect(PlanDirtyBadge.text(connected: false), PlanDirtyBadge.unsaved,
           "offline, saveToFile clears it (:592-594, behind offline()), so it means the plan "
           + "differs from the file; the badge said Unsent about a plan with no vehicle to send to")
    expect(PlanDirtyBadge.text(connected: true) != PlanDirtyBadge.text(connected: false),
           "one flag with two meanings needs two words, or half the readings are wrong")
}

checkPlanDirtyBadge()

func checkPlanFileReports() {
    expect(PlanFile.notSaved("route.plan").contains("route.plan"),
           "a refused save names the file the operator chose, not a path they never typed")
    expect(PlanFile.notSaved("route.plan").contains("Nothing was written"),
           "and says nothing was written, because PlanMasterController::saveToFile returns false "
           + "without creating the file and the head used to discard that bool entirely")
    expect(PlanFile.notLoaded("route.plan").contains("unchanged"),
           "a refused load says the existing plan survived, which is what the bridge's own "
           + "_aRefusedLoadLeavesTheExistingPlanAlone proves actually happens")
    expect(PlanFile.notSaved("a.plan") != PlanFile.notLoaded("a.plan"),
           "the two failures read differently, because losing a save and failing to open a file "
           + "call for different next moves")
}

checkPlanFileReports()

func checkClearNamesItsScope() {
    expect(!PlanClear.local.contains(PlanClear.vehicleWord),
           "plan.removeAll clears the mission, fences and rally points IN THE CONTROLLER ONLY, and "
           + "QGC reserves the word Mission for what is on the aircraft - QML's Clear Mission calls "
           + "removeAllFromVehicle. A label reading Clear or Clear Mission here would offer an "
           + "operator a control they would reasonably read as reaching the vehicle")
    expect(PlanClear.local.contains("Plan"),
           "so it names the plan, which is the local document this actually empties")
}

checkClearNamesItsScope()

func checkVehicleTrack() {
    func point(_ latitude: Double, _ longitude: Double) -> [String: Any] {
        ["latitude": latitude as NSNumber, "longitude": longitude as NSNumber]
    }
    let flying = VehicleTrack(["available": true as NSNumber, "recording": true as NSNumber,
                               "vehicleId": 1 as NSNumber, "generation": 3 as NSNumber,
                               "dropped": 0 as NSNumber, "count": 2 as NSNumber,
                               "points": [point(-35.36, 149.16), point(-35.361, 149.161)]])
    expect(flying.draws, "two positions make a trail")
    expect(flying.points.count == 2, "and both cross the bridge")
    expect(abs(flying.points[0].latitude - -35.36) < 1e-9, "in the order they were flown")
    expect(flying.notice, "", "a trail that has lost nothing says nothing")

    expect(!VehicleTrack(["available": true as NSNumber, "count": 1 as NSNumber,
                          "points": [point(-35.36, 149.16)]]).draws,
           "one position is not a line, and drawing it would put a dot where the vehicle marker "
           + "already is")
    expect(!VehicleTrack(["available": false as NSNumber, "count": 2 as NSNumber,
                          "points": [point(0, 0), point(1, 1)]]).draws,
           "and a trail from no vehicle is not drawn whatever it carries")

    let trimmed = VehicleTrack(["available": true as NSNumber, "dropped": 12 as NSNumber,
                                "count": 500 as NSNumber,
                                "points": [point(1, 1), point(2, 2)]])
    expect(trimmed.notice, "Trail trimmed \u{2014} showing the last 500 positions of this flight.",
           "a trail that starts mid-flight says why; without it a trimmed trail reads as a lost "
           + "link, and the count is the core's number rather than a cap copied into this head")

    expect(VehicleTrack.none.points.isEmpty && !VehicleTrack.none.draws
           && VehicleTrack.none.notice.isEmpty,
           "before the first read there is no trail, no drawing and nothing to explain")
    expect(VehicleTrack(["points": [["latitude": 1 as NSNumber]]]).points.isEmpty,
           "a point missing half its coordinate is dropped rather than read as a zero, which "
           + "would draw the trail through Null Island")
    expect(VehicleTrack(["vehicleId": NSNull()]).vehicleId == nil,
           "no vehicle carries no id; the core sends null and this head must not read it as 0")
}

checkVehicleTrack()

func checkVehicleMarker() {
    expect(VehicleMarker(latitude: nil, longitude: 149.16, heading: 0) == nil,
           "a marker needs both halves of a coordinate")
    expect(VehicleMarker(latitude: Double.nan, longitude: 149.16, heading: 0) == nil,
           "and they have to be numbers")

    guard let north = VehicleMarker(latitude: -35.36, longitude: 149.16, heading: 0) else {
        return expect(false, "a real coordinate makes a marker")
    }
    expect(north.rotationRadians == 0, "a heading of north does not rotate the marker")
    expect(north.hasHeading, "and it knows it has a heading")

    guard let east = VehicleMarker(latitude: -35.36, longitude: 149.16, heading: 90),
          let west = VehicleMarker(latitude: -35.36, longitude: 149.16, heading: 270) else {
        return expect(false, "east and west make markers")
    }
    expect(abs(east.rotationRadians + .pi / 2) < 1e-9,
           "a heading runs clockwise while the layer rotates anticlockwise, so east is negative")
    expect(abs(west.rotationRadians + 3 * .pi / 2) < 1e-9, "and west is three quarters the other way")

    guard let wrapped = VehicleMarker(latitude: -35.36, longitude: 149.16, heading: 450),
          let negative = VehicleMarker(latitude: -35.36, longitude: 149.16, heading: -90) else {
        return expect(false, "out of range headings make markers")
    }
    expect(abs(wrapped.rotationRadians - east.rotationRadians) < 1e-9, "450 degrees is 90")
    expect(abs(negative.rotationRadians - west.rotationRadians) < 1e-9, "and -90 is 270")

    guard let headless = VehicleMarker(latitude: -35.36, longitude: 149.16, heading: nil) else {
        return expect(false, "a vehicle without a heading still has a position")
    }
    expect(!headless.hasHeading, "a vehicle reporting no heading is drawn as a dot")
    expect(headless.rotationRadians == 0, "and is not rotated to an invented direction")
}

checkVehicleMarker()

func checkCameraChoice() {
    let known = CameraChoice(json: [
        "cameraBrand": "Canon", "cameraModel": "S100 PowerShot",
        "cameraBrandList": ["Manual (no camera specs)", "Custom Camera", "Canon", "GoPro"],
        "cameraModelList": ["S100 PowerShot", "EOS-M 22mm"],
    ])
    expect(known.canChooseBrand, "several brands can be chosen between")
    expect(known.canChooseModel, "and several models")
    expect(known.describes, "Canon S100 PowerShot", "the camera reads as brand and model")

    let manual = CameraChoice(json: [
        "cameraBrand": "Manual (no camera specs)", "cameraModel": "",
        "cameraBrandList": ["Manual (no camera specs)", "Canon"],
        "cameraModelList": [],
    ])
    expect(!manual.canChooseModel, "a brand with no models offers no model picker")
    expect(manual.describes, "Manual (no camera specs)", "and reads as just the brand")

    let single = CameraChoice(json: [
        "cameraBrand": "GoPro", "cameraModel": "Hero 4",
        "cameraBrandList": ["GoPro"], "cameraModelList": ["Hero 4"],
    ])
    expect(!single.canChooseBrand, "one brand is not a choice")
    expect(!single.canChooseModel, "nor is the one model already selected")

    expect(CameraChoice.empty.describes, "No camera", "an item with no camera says so")
    expect(!CameraChoice.empty.canChooseBrand, "and offers nothing to pick")
}

checkCameraChoice()

func checkAltitudeMode() {
    expect([AltitudeMode.mixedRaw, AltitudeMode.relativeRaw, AltitudeMode.absoluteRaw,
            AltitudeMode.calcAboveTerrainRaw, AltitudeMode.terrainFrameRaw,
            AltitudeMode.unrelatedRaw].map(String.init).joined(separator: ","),
           "0,1,2,3,4,5", "each mode is the number of its C++ enum case, in that order")

    func offer(_ raw: Int, _ title: String, enabled: Bool = true, reason: String = "") -> [String: Any] {
        ["raw": raw as NSNumber, "title": title, "help": "h",
         "current": false as NSNumber, "enabled": enabled as NSNumber, "reason": reason]
    }
    let listed = AltitudeMode.offers(["modes": [
        offer(AltitudeMode.relativeRaw, "Relative To Launch"),
        offer(AltitudeMode.absoluteRaw, "AMSL"),
        offer(AltitudeMode.calcAboveTerrainRaw, "Calculated Above Terrain",
              enabled: false, reason: "Add a mission item first."),
    ]])
    expect(listed.count == 3, "the head shows the modes the core offered and no others")
    expect(AltitudeMode.offers(["modes": [["title": "no raw"]]]).isEmpty,
           "a mode with no raw is dropped: the raw is the number written back through "
           + "plan.missionController, so an entry without one could only write the wrong mode")

    expect(AltitudeMode.choosable(listed).map(\.raw).map(String.init).joined(separator: ","),
           "1,2", "only the modes the core enabled can be picked; Terrain Frame on a firmware "
           + "that cannot hold it is not in the list at all, and this head used to offer all five "
           + "to every vehicle")
    expect(AltitudeMode.refusal(listed, raw: AltitudeMode.relativeRaw) == nil,
           "an enabled mode is written without complaint")
    expect(AltitudeMode.refusal(listed, raw: AltitudeMode.calcAboveTerrainRaw) ?? "",
           "Add a mission item first.",
           "and a disabled one is refused in the core's own words rather than silently ignored")
    expect(AltitudeMode.refusal(listed, raw: AltitudeMode.terrainFrameRaw) ?? "",
           "This vehicle does not offer that altitude mode.",
           "a mode the core never offered is refused too, which is the direction that matters: "
           + "the write is the gate, not the picker")

    expect(AltitudeMode.read(4 as NSNumber) == AltitudeMode.terrainFrameRaw,
           "the mode crosses the bridge as the number of its C++ enum case")
    expect(AltitudeMode.read("AltitudeModeTerrainFrame") == AltitudeMode.none,
           "and a name where a number belongs is no mode at all, which is what broke the pickers")
    expect(AltitudeMode.read(nil) == AltitudeMode.none, "so is a missing one")

    expect(AltitudeMode.title(for: AltitudeMode.absoluteRaw, in: listed), "AMSL",
           "a mode reads as the core's words")
    expect(AltitudeMode.title(for: 9, in: listed), "Mode 9",
           "an unknown mode is shown as sent rather than hidden")
    expect(AltitudeMode.title(for: AltitudeMode.none, in: listed), "",
           "but an item that carries no altitude at all names no mode")
    expect(AltitudeMode.title(for: AltitudeMode.unrelatedRaw, in: listed), "",
           "and neither does AltitudeModeNone, which QGC draws as an empty string rather than "
           + "\u{201C}Mode 5\u{201D}")
}

checkAltitudeMode()

func checkPlanSummary() {
    let read = MissionSummary(["available": true as NSNumber, "reason": "",
                               "altitudeRange": ["text": "120 m to 340 m"],
                               "rows": [["id": "distance", "label": "Distance", "value": "7.05 km"],
                                        ["id": "time", "label": "Time", "value": "12:30"],
                                        ["id": "hover", "label": "Hover", "value": "1.20 km"],
                                        ["id": "batteries", "label": "Batteries", "value": "2"],
                                        ["id": "furthest", "label": "Furthest from launch",
                                         "value": "2.10 km"]]])
    expect(read.value(MissionSummary.distance) ?? "", "7.05 km",
           "the core spells the distance, crossing into kilometres past a thousand metres. This "
           + "head formatted it and never crossed over, so a seven kilometre mission read as "
           + "7047 m -- and 23120 ft on imperial, where QGC says 4.38 mi")
    expect(read.value(MissionSummary.time) ?? "", "12:30", "and the duration")
    expect(read.value("planned") == nil,
           "a row the controller could not compute is absent rather than zero: needing no "
           + "batteries and having no battery model are different facts, and both would read 0")
    expect(read.extraRows.map(\.label).joined(separator: ","), "Hover,Batteries",
           "the strip pulls out the three it draws itself and the rest follow, so the same fact "
           + "is never printed twice")
    expect(read.altitudeRange, "120 m to 340 m", "the altitude range is the core's sentence")
    expect(read.describes, "a plan with rows has something to summarise")

    let multirotor = MissionSummary(["available": true as NSNumber, "reason": "",
                                     "rows": [["id": "distance", "label": "Distance",
                                               "value": "14.11 km"],
                                              ["id": "planned", "label": "Planned",
                                               "value": "14.11 km"],
                                              ["id": "time", "label": "Time", "value": "47:27"],
                                              ["id": "hover", "label": "Hover",
                                               "value": "14.11 km"],
                                              ["id": "furthest",
                                               "label": "Furthest from launch",
                                               "value": "14.11 km"]]])
    expect(multirotor.extraRows.isEmpty,
           "a figure already on the strip is not printed again under another name. A multirotor "
           + "hovers the whole way, so its Hover distance IS its total and its Planned distance "
           + "is that same figure a third time -- measured, the strip printed 14.11 km four "
           + "times across a sidebar that then wrapped every cell, rendering the distance as "
           + "\"14.\" over \"11\" over \"km\"")
    expect(!read.extraRows.isEmpty,
           "and the case above keeps two rows, without which an extraRows that always answered "
           + "empty would satisfy the assertion here for the wrong reason")

    expect(MissionSummaryRow(["label": "Distance", "value": ""]) == nil,
           "a row with an empty value is dropped rather than drawn as a label with nothing "
           + "beside it")
    expect(MissionSummaryRow(["value": "7 km"]) == nil, "and one with no label at all")

    let empty = MissionSummary(["available": false as NSNumber, "rows": [],
                                "reason": "This plan has no items yet."])
    expect(!empty.describes, "an empty plan has nothing to summarise")
    expect(empty.reason, "This plan has no items yet.", "and the core says why")
    expect(empty.altitudeRange, "", "with no altitude range rather than an invented one")
    expect(MissionSummary([:]).describes == false, "an empty read describes nothing")
}

checkPlanSummary()

func checkSummaryRowsSurviveALabelChange() {
    let translated = MissionSummary(["available": true as NSNumber, "reason": "",
                                     "rows": [["id": "distance", "label": "Strecke",
                                               "value": "7.05 km"],
                                              ["id": "time", "label": "Zeit", "value": "12:30"],
                                              ["id": "furthest", "label": "Am weitesten vom Start",
                                               "value": "2.10 km"]]])
    expect(translated.value(MissionSummary.distance) ?? "", "7.05 km",
           "the curated rows are found by the core's stable id, so a localised label changes "
           + "nothing. Keyed on the English label instead -- which is what this head did until "
           + "48ce5fb86 gave every row an id -- all three lookups returned nil the moment the "
           + "core spelled a label in another language, and the survey polygon that went "
           + "undrawn for exactly that reason is the precedent")
    expect(translated.timeText, "12:30", "and the duration is drawn rather than an em dash")
    expect(translated.extraRows.isEmpty,
           "and none of the three falls through to the extra rows, which is what a failed "
           + "lookup looks like from the operator's side: every figure drawn twice over")

    let older = MissionSummary(["available": true as NSNumber, "reason": "",
                                "rows": [["label": "Distance", "value": "7.05 km"]]])
    expect(older.value("Distance") ?? "", "7.05 km",
           "a row with no id at all keeps its label as one. The label WAS the identifier until "
           + "the core added ids, so falling back to it is the previous behaviour rather than "
           + "an invented value -- and a row dropped for lacking an id would take a figure off "
           + "the screen, which is worse than a lookup that misses")
}
checkSummaryRowsSurviveALabelChange()

func checkSurveyStats() {
    func stats(_ overrides: [String: Any]) -> SurveyStats {
        SurveyStats(["available": true as NSNumber, "shotsText": "1043",
                     "intervalText": "0.8 s", "footprintText": "15.2 \u{00D7} 6.8 m",
                     "tooFast": false as NSNumber, "warning": "",
                     "areaText": "89999 m\u{00B2}", "distanceText": "7261 m"]
            .merging(overrides) { _, new in new })
    }

    expect(!SurveyStats.none.describes, "an item that is not a survey describes nothing")

    let live = stats([:])
    expect(live.describes, "a real survey does, because the core says the item is one")
    expect(live.shotsText, "1043", "the photo count is the core's")
    expect(live.intervalText, "0.8 s", "so is the interval")
    expect(live.footprintText, "15.2 \u{00D7} 6.8 m", "and each photo's ground footprint")
    expect(live.areaText, "89999 m\u{00B2}",
           "the area is the core's sentence too, superscript and all, since b607aa049 gave it "
           + "QGC's formatMeasure rules")
    expect(live.distanceText, "7261 m", "and the distance alongside it")

    let absent = stats(["areaText": "\u{2014}", "distanceText": "\u{2014}"])
    expect(absent.areaText, "\u{2014}", "no area is not zero area")
    expect(absent.distanceText, "\u{2014}", "nor is no distance")

    let strained = stats(["tooFast": true as NSNumber,
                          "warning": "The camera needs 2.00 s between shots but the survey asks "
                                     + "for 0.84 s."])
    expect(strained.tooFast, "the core decides a camera cannot keep up")
    expect(strained.warning.contains("2.00 s"), "and its sentence names what the camera needs")
    expect(strained.warning.contains("0.84 s"), "alongside what the survey asks for")

    expect(SurveyStats(["available": true as NSNumber, "isSurvey": false as NSNumber,
                        "shotsText": "126", "areaText": "\u{2014}",
                        "distanceText": "3.25 km"]).describes,
           "and so does a structure scan, which is the third member of the class and the one "
           + "abfa9d123 did not measure -- it covers no area at all, so areaSquareMetres is 0 "
           + "and the core renders the em dash, and a head reading areaText to decide would "
           + "have hidden 126 photos over 3.25 km behind a blank")
    expect(SurveyStats(["available": true as NSNumber, "isSurvey": false as NSNumber,
                        "shotsText": "27", "areaText": "15000 m\u{b2}"]).describes,
           "a corridor scan describes its camera work too. 647cd7193 stopped the core gating "
           + "available on is_survey, and this head had the same premise one layer out -- it "
           + "asked whether the item was a survey before it would even read the view, so a "
           + "corridor with 27 shots over 15000 m\u{b2} drew nothing. describes reads what the "
           + "producer decided and never the item's kind")
    expect(SurveyStats([:]).describes == false,
           "a read that returned nothing describes nothing rather than an empty survey")
}

func checkMeasure() {
    expect(Measure.defaultUnits, "m", "the unit an item falls back to when the core names none")
    expect(Measure.unreported, "\u{2014}",
           "and what a measure the core did not spell reads as. Measure used to carry the "
           + "FORMATTER too -- reading(), format(), pretty() and a hundred-metre threshold "
           + "mirroring read.rs -- and every one now has no caller in macos/Sources. The "
           + "battery, the launch altitude and the rally altitude each took the core's own "
           + "string this evening, and the copy of the core's algorithm went with the last of "
           + "them")
}

checkMeasure()
checkSurveyStats()

func checkCalibration() {
    func side(_ key: String, _ title: String, _ stage: String, _ rotate: Bool = false) -> [String: Any] {
        ["key": key, "title": title, "stage": stage, "rotate": rotate as NSNumber]
    }
    func routine(_ id: String, _ title: String, _ method: String, _ args: [Any] = [],
                 _ blocked: Bool = false, _ enabled: Bool = true, _ description: String = "d",
                 _ warning: String = "") -> [String: Any] {
        ["id": id, "title": title, "invocation": "sensorsCal.\(method)", "arguments": args,
         "blocked": blocked as NSNumber, "enabled": enabled as NSNumber,
         "description": description, "warning": warning]
    }

    expect(!CalibrationState.disconnected.connected,
           "with no controller there is nothing to calibrate")
    expect(!CalibrationState([:]).connected, "and a read that returned nothing is not connected")

    let live = CalibrationState([
        "connected": true as NSNumber, "inProgress": true as NSNumber, "busy": true as NSNumber,
        "showsSides": true as NSNumber, "nextEnabled": true as NSNumber,
        "cancelEnabled": true as NSNumber, "progress": 33.4 as NSNumber, "progressText": "33%",
        "helpText": "Hold still", "statusText": "Rotate the vehicle",
        "needsAttention": "The accelerometer and compass both need calibrating.",
        "visibleSides": [side("Down", "Level", "done"),
                         side("Left", "Left side", "inProgress", true),
                         side("Right", "Right side", "waiting")],
        "routines": [routine("accelerometer", "Accelerometer", "calibrateAccel",
                             [false as NSNumber])],
    ])
    expect(live.busy, "a running calibration is busy")
    expect(live.progressText, "33%", "progress is the core's whole percent")
    expect(live.visibleSides.map(\.title).joined(separator: ","), "Level,Left side,Right side",
           "only the sides this calibration asks for are shown, in the vehicle's order")
    expect(live.visibleSides[0].stage == .done, "a finished side is done")
    expect(live.visibleSides[1].stage == .inProgress, "the one being held is in progress")
    expect(live.visibleSides[2].stage == .waiting, "and one not yet reached is waiting")
    expect(live.visibleSides[1].symbol, "arrow.triangle.2.circlepath",
           "a side that must be rotated says so rather than showing a plain arrow")
    expect(live.needsAttention, "The accelerometer and compass both need calibrating.",
           "and the core names both outstanding calibrations together")

    expect(CalibrationSide(side("X", "X", "somethingNew"))?.stage == .unknown,
           "a stage the core adds later is unknown rather than silently drawn as waiting")
    expect(CalibrationSide(side("X", "X", "somethingNew"))?.symbol ?? "", "circle",
           "and it still gets a glyph rather than nothing at all")
    expect(CalibrationSide(["title": "no key"]) == nil, "a side with no key is dropped")

    let accel = CalibrationRoutine(routine("accelerometer", "Accelerometer", "calibrateAccel",
                                           [false as NSNumber]))
    expect(accel?.invocation ?? "", "sensorsCal.calibrateAccel",
           "each routine names the controller method the core says to call")
    expect(accel?.arguments == [false], "the accelerometer carries its simple-calibration flag")
    expect(CalibrationRoutine(routine("gyro", "Gyro", "calibrateGyro"))?.arguments.isEmpty ?? false,
           "the others take none")
    expect(CalibrationRoutine(["id": "compass", "title": "Compass"]) == nil,
           "a routine with no invocation is dropped rather than starting nothing on a press")
    expect(CalibrationRoutine(["id": "compass", "invocation": ""]) == nil,
           "and so is one whose invocation is empty")
}

checkCalibration()

func checkAbout() {
    expect(AboutInfo.version(short: "5.4", build: "5.4.7"), "5.4 (5.4.7)",
           "a build that differs from the version is shown in brackets")
    expect(AboutInfo.version(short: "5.4", build: "5.4"), "5.4",
           "a build that matches is not repeated")
    expect(AboutInfo.version(short: "5.4", build: ""), "5.4", "a missing build is simply absent")
    expect(AboutInfo.version(short: "", build: "5.4.7"), "5.4.7",
           "and a bundle with only a build still names one")
    expect(AboutInfo.version(short: "", build: ""), "unknown",
           "a bundle with neither says so rather than showing an empty row")

    let read = AboutInfo.read(["CFBundleName": "Aircast QGC",
                               "CFBundleShortVersionString": "5.4", "CFBundleVersion": "5.4.7"])
    expect(read.versionText, "5.4 (5.4.7)", "the bundle is read as the app was built")
    expect(AboutInfo.read(nil).name, "Aircast QGC", "a missing dictionary still names the app")

    expect(HelpLink.all.count == 4, "the four QGC help destinations are offered")
    expect(HelpLink.all[0].host, "docs.qgroundcontrol.com",
           "each shows its host rather than a full URL")
    expect(HelpLink(name: "x", url: "http://discuss.px4.io/c/qgroundcontrol").host,
           "discuss.px4.io", "for http as well as https, and the path is dropped")
}

checkAbout()

func checkRadio() {
    func channel(_ index: Int, _ pwm: Int, _ live: Bool = true) -> [String: Any] {
        ["index": index as NSNumber, "label": "\(index + 1)", "value": pwm as NSNumber,
         "valueText": live ? "\(pwm)" : "\u{2014}",
         "fraction": (live ? Double(pwm - 1000) / 1000 : 0) as NSNumber, "live": live as NSNumber]
    }
    func stick(_ key: String, _ title: String, _ pwm: Int, _ mapped: Bool = true,
               _ reversed: Bool = false) -> [String: Any] {
        ["key": key, "title": title, "mapped": mapped as NSNumber, "value": pwm as NSNumber,
         "valueText": mapped ? "\(pwm)" : "Not mapped",
         "fraction": (Double(pwm - 1000) / 1000) as NSNumber, "reversed": reversed as NSNumber]
    }

    expect(!RadioState.disconnected.connected, "with no controller there is no radio page to fill")
    expect(!RadioState([:]).connected, "and a read that returned nothing is not connected")

    let state = RadioState([
        "connected": true as NSNumber, "channelCount": 16 as NSNumber,
        "summary": "16 channels reported, 8 carrying a signal.", "shortfall": "",
        "calibrating": false as NSNumber, "nextText": "Calibrate",
        "nextEnabled": true as NSNumber, "transmitterMode": 2 as NSNumber,
        "channels": [channel(0, 1500), channel(1, 1500), channel(2, 1000), channel(3, 1500),
                     channel(4, 1800), channel(5, 1000), channel(6, 1000), channel(7, 1800),
                     channel(8, 0, false), channel(9, 0, false)],
        "sticks": [stick("roll", "Roll", 1500), stick("pitch", "Pitch", 1500),
                   stick("yaw", "Yaw", 1500, true, true), stick("throttle", "Throttle", 1000)],
    ])
    expect(state.channelCount == 16, "the reported channel count is the core's")
    expect(state.liveChannels.count == 8,
           "only channels carrying a signal are listed; the silent ones are not drawn as empty bars")
    expect(state.summary, "16 channels reported, 8 carrying a signal.", "and the summary says both")
    expect(state.shortfall, "", "so nothing is wanting")
    expect(!state.calibrating, "an idle controller is not calibrating")
    expect(state.channels[0].fraction == 0.5, "centre sits in the middle of the bar")
    expect(state.channels[8].valueText, "\u{2014}",
           "a channel carrying nothing shows a dash rather than a zero")

    expect(state.sticks.map(\.title).joined(separator: ","), "Roll,Pitch,Yaw,Throttle",
           "the four sticks are named in the order a pilot reads them")
    expect(state.sticks[3].valueText, "1000", "throttle down reads as its pulse width")
    expect(state.sticks[2].reversed, "a reversed channel is marked")
    expect(!state.sticks[0].reversed, "and an unreversed one is not")
    expect(RadioStick(stick("roll", "Roll", 0, false))?.valueText ?? "", "Not mapped",
           "a stick with no channel assigned says so instead of showing a dash")

    let thin = RadioState([
        "connected": true as NSNumber, "channelCount": 4 as NSNumber,
        "shortfall": "At least 5 channels are needed to fly; the transmitter reports 4.",
    ])
    expect(thin.shortfall, "At least 5 channels are needed to fly; the transmitter reports 4.",
           "and the page says so in the pilot's terms")

    expect(RadioState(["connected": true as NSNumber,
                       "summary": "No transmitter is being heard."]).summary,
           "No transmitter is being heard.",
           "a vehicle with no transmitter says that rather than reporting zero channels")

    expect(RadioState([:]).transmitterMode == 2,
           "a reply with no transmitter mode reads as mode 2, which is where QGC's own controller "
           + "starts \u{2014} mode 0 is not a mode any transmitter has")
    expect(RadioState(["transmitterMode": 1 as NSNumber]).transmitterMode == 1,
           "and a mode the core does report is taken as given")

    expect(RadioChannel(["label": "1"]) == nil, "a channel with no index is dropped")
    expect(RadioStick(["title": "Roll"]) == nil, "and so is a stick with no key")
}

checkRadio()

func checkVideoStatus() {
    expect(VideoStatus.unavailable.summary, "",
           "an unread status says nothing rather than claiming this build cannot show video, "
           + "which is now the core's sentence to write")

    let live = VideoStatus([
        "available": true as NSNumber, "gstreamer": true as NSNumber,
        "decoding": true as NSNumber, "recording": true as NSNumber,
        "anyConnecting": false as NSNumber,
        "multipleSources": true as NSNumber, "activeSource": 1 as NSNumber,
        "summary": "Streaming and recording.",
        "cameras": [
            ["slot": 0 as NSNumber, "title": "Camera 1", "status": "rtsp://one",
             "connecting": false as NSNumber, "recording": true as NSNumber,
             "configured": true as NSNumber],
            ["slot": 1 as NSNumber, "title": "Camera 2", "status": "No stream URL",
             "connecting": false as NSNumber, "recording": false as NSNumber,
             "configured": false as NSNumber],
        ],
    ])
    expect(live.summary, "Streaming and recording.",
           "the summary is the core's sentence, not one this head assembles from flags")
    expect(live.cameras.count == 2, "each camera slot is carried across")
    expect(live.cameras[0].title, "Camera 1", "with the core's title rather than a slot number plus one")
    expect(live.configuredCameras.map(\.slot) == [0],
           "and the core decides which are configured, from the status text it also owns")

    expect(VideoCamera(["title": "Camera 1"]) == nil,
           "a camera with no slot is dropped, because the slot is what the row is keyed by")
    expect(VideoStatus([:]).cameras.isEmpty, "an empty answer is no cameras")

    expect(live.offersSwitch, "with more than one source the next camera can be asked for")
    expect(live.activeCameraTitle, "Camera 2",
           "and the switch is labelled with the camera at activeSource, not the first in the list")

    let cameras = live.cameras.map { camera -> [String: Any] in
        ["slot": camera.slot as NSNumber, "title": camera.title, "status": camera.status,
         "connecting": camera.connecting as NSNumber, "recording": camera.recording as NSNumber,
         "configured": camera.configured as NSNumber]
    }
    let single = VideoStatus(["multipleSources": false as NSNumber,
                              "activeSource": 1 as NSNumber, "cameras": cameras])
    expect(!single.offersSwitch,
           "one switchable source offers no switch, even though a camera sits at activeSource")
    expect(single.activeCameraTitle, "Camera 2",
           "the active camera is still known; it is the switch that is withheld")

    let unnamed = VideoStatus(["multipleSources": true as NSNumber,
                               "activeSource": 7 as NSNumber, "cameras": cameras])
    expect(!unnamed.offersSwitch,
           "a source the core sent no camera for is not offered under an invented name")
    expect(unnamed.activeCameraTitle, "",
           "and the head writes no \"Camera 8\" of its own; that fallback is the core's")
}
checkVideoStatus()

func checkVideoSources() {
    let live = "[{\"name\":\"\",\"source\":\"RTSP Video Stream\",\"url\":\"\"},"
        + "{\"name\":\"0.0.0.0:5691\",\"source\":\"UDP h.264 Video Stream\",\"url\":\"\"},"
        + "{\"name\":\"\",\"source\":\"Video Stream Disabled\",\"url\":\"\"}]"

    func camera(_ slot: Int, enabled: Bool, configured: Bool) -> VideoCamera? {
        VideoCamera(["slot": slot as NSNumber, "title": "Camera \(slot + 1)", "status": "",
                     "enabled": enabled as NSNumber, "configured": configured as NSNumber])
    }
    let answered = [camera(1, enabled: true, configured: false),
                    camera(2, enabled: true, configured: false),
                    camera(3, enabled: false, configured: false)].compactMap { $0 }

    let sources = VideoSources.decode(live, cameras: answered)
    expect(sources.count == 3, "every configured slot is read")
    expect(sources[0].title, "Camera 1", "an unnamed slot is named by its number")
    expect(sources[1].title, "0.0.0.0:5691", "a named one keeps its name")
    expect(!sources[2].enabled, "a disabled slot is off")
    expect(sources[2].summary, "Off", "and says so rather than complaining about an address")
    expect(sources[0].misconfigured, "an enabled slot with no address cannot work")
    expect(!sources[2].misconfigured, "a disabled one is not misconfigured, just off")
    expect(sources[1].summary, "No address", "which is what the row reports")

    let offset = VideoSources.decode(live, cameras: answered)
    expect(offset[0].enabled && !offset[0].configured,
           "extra slot 0 takes its answer from camera slot 1: VideoSettings numbers the main "
           + "videoSource fact as slot 0, so reading camera slot 0 here would report the wrong "
           + "camera's state for every row")

    let webcam = "[{\"name\":\"\",\"source\":\"FaceTime HD Camera\",\"url\":\"\"}]"
    let attached = VideoSources.decode(webcam,
                                       cameras: [camera(1, enabled: true, configured: true)]
                                           .compactMap { $0 })
    expect(!attached[0].misconfigured,
           "a source that needs no address is not broken for having none; this head used to call "
           + "every empty url misconfigured, so a webcam, a Herelink and a 3DR Solo were each "
           + "shown as faulty and offered for repair")
    expect(attached[0].summary, "FaceTime HD Camera",
           "and the row names it rather than reporting a blank address")

    let unanswered = VideoSources.decode(live)
    expect(!unanswered[0].misconfigured && !unanswered[2].misconfigured,
           "a slot the core has not answered for yet accuses nothing; an absent reply must not "
           + "read as a fault, which is the direction that puts a repair button on a good camera")

    expect(VideoSources.decode("not json").isEmpty, "a corrupt setting yields no slots, not a crash")
    expect(VideoSources.decode("").isEmpty, "nor does an empty one")

    expect(VideoSources.looksLikeAddress("0.0.0.0:5691"), "a host and port is an address")
    expect(VideoSources.looksLikeAddress("rtsp://camera/live"), "so is a URL")
    expect(!VideoSources.looksLikeAddress("Front camera"), "a human name is not")
    expect(!VideoSources.looksLikeAddress("nose:cam"), "nor is a colon without a port number")
    expect(!VideoSources.looksLikeAddress(""), "nor is nothing")

    let repaired = VideoSources.repairs(sources[1])
    expect(repaired?.url ?? "", "0.0.0.0:5691",
           "an address typed into the name is offered as the address")
    expect(repaired?.name ?? "?", "", "and stops being the name")
    expect(VideoSources.repairs(sources[0]) == nil,
           "a slot with no address anywhere has nothing to move")
    expect(VideoSources.repairs(sources[2]) == nil, "and a disabled slot is left alone")

    var fixed = sources[1]
    fixed.url = "0.0.0.0:5691"
    let updated = VideoSources.replacing(sources, at: 1, with: fixed)
    expect(updated.count == 3, "replacing a slot keeps the others")
    expect(updated[1].url, "0.0.0.0:5691", "and changes the one asked for")
    expect(updated[0].url, "", "leaving its neighbours alone")
    expect(VideoSources.encode(updated).contains("0.0.0.0:5691"),
           "the encoded setting carries the address back to the vehicle settings")
}

checkVideoSources()

func checkCameraControl() {
    expect(!CameraControl.absent.present, "no camera is not present")

    let camera = CameraControl([
        "present": true as NSNumber, "title": "Simulated Camera", "model": "Simulated Camera",
        "vendor": "QGC", "mode": 0 as NSNumber, "modeKnown": true as NSNumber,
        "modeText": "Photo", "isRecording": false as NSNumber,
        "isTakingPhoto": false as NSNumber, "stateText": "Idle", "clockText": "00:00:00",
        "storageStatus": 2 as NSNumber, "storageText": "Not reported",
        "shots": 0 as NSNumber, "shotsText": "00000",
        "batteryRemaining": -1 as NSNumber, "batteryText": "",
        "hasZoom": false as NSNumber, "zoomLevel": 1.0 as NSNumber,
        "canRecord": true as NSNumber, "canPhoto": true as NSNumber,
        "hasModes": true as NSNumber,
    ])
    expect(camera.present && camera.title == "Simulated Camera",
           "the camera card takes its title from the core, which prefers the model over the vendor")
    expect(camera.modeText, "Photo", "and the mode's wording, rather than mapping an enum here")
    expect(camera.shotsText, "00000",
           "including the five-digit shot counter, which was a format string in this head")
    expect(camera.mode == CameraControl.photoMode,
           "the numeric mode still comes across, because the picker writes it back")

    expect(CameraControl([:]).present == false,
           "an empty answer is no camera rather than a present one with blank fields")
}

    let busy = CameraControl(["present": true as NSNumber, "hasModes": true as NSNumber,
                              "canChangeMode": false as NSNumber])
    expect(busy.hasModes && !busy.canChangeMode,
           "a camera mid-capture still HAS modes and will not accept one now - core-rs video.rs "
           + "computes canChangeMode as present && hasModes && can_change_mode(mode, photo, video), "
           + "and this head used to gate the mode change on hasModes, so the picker stayed live "
           + "while recording and the command fired into a camera that would refuse it")
    expect(!CameraControl(["present": true as NSNumber, "hasModes": true as NSNumber]).canChangeMode,
           "and an absent canChangeMode reads as not-now rather than go-ahead, because the "
           + "permissive direction here sends a command the camera rejects")
    expect(CameraControl(["present": true as NSNumber, "hasModes": true as NSNumber,
                          "canChangeMode": true as NSNumber]).canChangeMode,
           "an idle camera that has modes accepts one")

checkCameraControl()

func checkLogReplayLink() {
    let empty = LinkConfig(["index": 0 as NSNumber, "type": "logReplay", "name": "Replay",
                            "editing": "logFile", "displaySummary": "No log chosen"])
    expect(empty?.editing == .logFile, "a log replay link is edited by choosing a file")
    expect(empty?.displaySummary ?? "", "No log chosen",
           "and the core says so rather than leaving an empty summary")

    let chosen = LinkConfig(["index": 0 as NSNumber, "type": "logReplay", "name": "Replay",
                             "editing": "logFile", "filename": "/Users/pilot/logs/flight.tlog",
                             "logFileName": "flight.tlog", "displaySummary": "Log Replay"])
    expect(chosen?.logFileName ?? "", "flight.tlog",
           "a chosen log shows its file name, because the full path does not fit the row")
    expect(chosen?.filename ?? "", "/Users/pilot/logs/flight.tlog",
           "while the whole path is kept for the tooltip")
}

checkLogReplayLink()

func checkMissionVehicle() {
    let copter = MissionVehicle(firmware: "ArduPilot", type: "Quadrotor", multiRotor: true, vtol: false, apmFirmware: false)
    expect(copter.showsHoverSpeed, "a multirotor hovers between waypoints")
    expect(!copter.showsCruiseSpeed, "and never cruises, so asking a cruise speed would be noise")

    let plane = MissionVehicle(firmware: "PX4 Pro", type: "Fixed Wing", multiRotor: false, vtol: false, apmFirmware: false)
    expect(plane.showsCruiseSpeed, "a plane cruises")
    expect(!plane.showsHoverSpeed, "and cannot hover")

    let vtol = MissionVehicle(firmware: "PX4 Pro", type: "VTOL", multiRotor: false, vtol: true, apmFirmware: false)
    expect(vtol.showsCruiseSpeed, "a VTOL does both")
    expect(vtol.showsHoverSpeed, "so it needs both speeds")

    expect(copter.isDescribed, "a named firmware and type describe the vehicle")
    expect(!MissionVehicle.unknown.isDescribed, "an unknown vehicle describes nothing")
    expect(MissionVehicle.unknown.showsAnything,
           "even unknown, a cruise speed is still worth asking for the time estimate")
}

checkMissionVehicle()

func checkGeoTagJob() {
    var job = GeoTagJob()
    expect(!job.canStart, "with neither a log nor images there is nothing to tag")
    expect(job.destination, "", "and nowhere to put the result")

    job.imageDirectory = "/Users/pilot/Survey"
    expect(!job.canStart, "images alone carry no positions")
    expect(job.destination, "/Users/pilot/Survey/TAGGED",
           "without a chosen folder the tagged images sit beside the originals")

    job.logFile = "/Users/pilot/logs/flight.ulg"
    expect(job.canStart, "a log and images are all it needs")

    job.saveDirectory = "/Volumes/Card/out"
    expect(job.destination, "/Volumes/Card/out", "a chosen folder wins over the default")

    expect(GeoTagJob.abbreviate("/Users/pilot/Survey", home: "/Users/pilot"), "~/Survey",
           "a path under home reads from home")
    expect(GeoTagJob.abbreviate("/Users/pilot", home: "/Users/pilot"), "~", "home itself is ~")
    expect(GeoTagJob.abbreviate("/Volumes/Card", home: "/Users/pilot"), "/Volumes/Card",
           "a path elsewhere is left alone")
    expect(GeoTagJob.abbreviate("/Users/pilotage/x", home: "/Users/pilot"), "/Users/pilotage/x",
           "a longer name that merely starts with home is not under it")

    expect(GeoTagJob.shortPath("/Users/pilot/Survey", home: "/Users/pilot"), "~/Survey",
           "a short path is shown whole")
    expect(GeoTagJob.shortPath("/Users/pilot/2026/kyiv/survey", home: "/Users/pilot"),
           "\u{2026}/kyiv/survey", "a deep one keeps the two components that identify it")
    expect(GeoTagJob.shortPath("/Volumes/Card/dcim/100MSDCF/a.jpg", home: "/Users/pilot"),
           "\u{2026}/100MSDCF/a.jpg", "off home too")

    job.progress = 100
    job.running = false
    expect(job.finished, "a finished run has reached the end and stopped")
    job.running = true
    expect(!job.finished, "a running one has not")

    var failing = GeoTagJob()
    failing.logFile = "/a.ulg"
    failing.imageDirectory = "/images"
    failing.running = true
    failing.progress = 20
    expect(failing.busy, "a running job with nothing wrong is busy")
    expect(!failing.failed, "and has not failed")

    failing.errorMessage = "Geotagging failed."
    expect(!failing.busy, "an error stops it being busy, however the worker thread lingers")
    expect(failing.failed, "and marks it failed, so the button can offer another go")

    var warned = GeoTagJob()
    warned.imageDirectory = "/images"
    warned.errorMessage = "Images have already been tagged."
    expect(!warned.failed, "a warning before the run is not a failure")
    expect(!warned.busy, "nor is it busy")
}

checkGeoTagJob()

func checkMavlinkMessage() {
    func message(_ overrides: [String: Any]) -> MavlinkMessage? {
        MavlinkMessage(["path": "mavlinkInspector.systems.0.messages.0", "index": 0 as NSNumber,
                        "id": 30 as NSNumber, "compId": 1 as NSNumber, "name": "ATTITUDE",
                        "title": "ATTITUDE", "count": 12 as NSNumber, "rateText": "10.0 Hz",
                        "targetRateHz": 0 as NSNumber, "targetRateTitle": "Default",
                        "selected": false as NSNumber]
            .merging(overrides) { _, new in new })
    }

    expect(message([:])?.rateText ?? "", "10.0 Hz", "the rate is the core's sentence")
    expect(message(["rateText": "\u{2014}"])?.rateText ?? "", "\u{2014}",
           "and a message that has never repeated shows the core's dash")
    expect(message([:])?.countText ?? "", "12", "the count is drawn beside it")
    expect(message([:])?.targetRateTitle ?? "", "Default",
           "and the requested rate reads as the core titles it")

    expect(message(["path": ""]) == nil, "a message with no path cannot be identified and is dropped")
    expect(message(["name": ""]) == nil, "nor is a nameless one a message")

    let first = message(["path": "mavlinkInspector.systems.0.messages.2", "id": 262 as NSNumber,
                         "compId": 100 as NSNumber, "name": "CAMERA_CAPTURE_STATUS",
                         "title": "CAMERA_CAPTURE_STATUS (comp 100)"])
    let second = message(["path": "mavlinkInspector.systems.0.messages.3", "id": 262 as NSNumber,
                          "compId": 101 as NSNumber, "name": "CAMERA_CAPTURE_STATUS",
                          "title": "CAMERA_CAPTURE_STATUS (comp 101)"])
    expect(first?.id != second?.id,
           "two cameras sending the same message id are two rows, because identity is the path "
           + "\u{2014} keying on the message id gave SwiftUI duplicate identities and crashed the "
           + "Android head")
    expect(first?.messageId == second?.messageId,
           "even though the message id they report is the same one")
    expect(first?.title ?? "", "CAMERA_CAPTURE_STATUS (comp 100)",
           "and the core says which component each row is, so the list is readable")
    expect(message([:])?.title ?? "", "ATTITUDE",
           "while a message only one component sends keeps its bare name")

    expect(MavlinkField(json: ["name": "roll", "type": "float", "value": "-0.01"])?.value ?? "",
           "-0.01", "a field carries the value the vehicle sent, unrounded")
    expect(MavlinkField(json: ["type": "float"]) == nil, "a field without a name is not one")
}

checkMavlinkMessage()

func checkFrameSetup() {
    expect(!FrameSetup.unknown.known, "no vehicle reports no frame")
    let quad = FrameSetup(vehicleType: "Quadrotor", motorCount: 4)
    expect(quad.known, "a reported type and motor count is a frame")
    expect(quad.motorText, "4 motors", "motors are counted")
    expect(FrameSetup(vehicleType: "Single", motorCount: 1).motorText, "1 motor",
           "and one of them is singular")
    expect(FrameSetup(vehicleType: "Quadrotor", motorCount: 0).motorText, "—",
           "a vehicle that reports no motors says nothing rather than zero")

    expect(FrameSetup.needsFrameClass("0"), "frame class 0 is no airframe at all")
    expect(!FrameSetup.needsFrameClass("1"), "any other class is a chosen airframe")
    expect(!FrameSetup.needsFrameClass(nil), "a firmware without the parameter is not misconfigured")
}

checkFrameSetup()

func checkVehicleMessages() {
    func item(_ time: String, _ level: String, _ severity: String, _ text: String,
              component: Int? = nil) -> [String: Any] {
        var made: [String: Any] = ["time": time, "level": level, "severity": severity, "text": text]
        if let component { made["component"] = component as NSNumber }
        return made
    }

    let read = VehicleMessage.list([
        item("23:24:17.897", "error", "Critical", "PreArm: GPS 1: not healthy"),
        item("23:23:26.733", "normal", "Info", "Frame: QUAD/PLUS", component: 1),
        item("23:23:26.733", "warning", "Warning", "something odd"),
    ])
    expect(read.count == 3, "every message the core lists is carried across, newest first")
    expect(read[0].text, "PreArm: GPS 1: not healthy",
           "the core has already taken the tags and the severity word off the text")
    expect(read[0].stamp, "23:24:17",
           "the row shows the clock and drops the milliseconds, which is a rendering choice and "
           + "the only reason this head still trims anything")
    expect(read[0].level == .error, "a critical message is an error")
    expect(read[1].level == .normal, "an info message is not")
    expect(read[2].level == .warning, "and a warning is its own level")
    expect(read[1].component == 1, "a message from a named component says which")
    expect(read[0].component == nil, "and one from the only component says nothing")

    expect(VehicleMessage.list(nil).isEmpty, "a vehicle that has said nothing has no messages")
    expect(VehicleMessage.list([["time": "1", "level": "normal"]]).isEmpty,
           "a message with no text is dropped rather than shown blank")
    expect(VehicleMessage.list([item("1", "normal", "Info", "")]).isEmpty,
           "and so is one whose text came back empty, which is a blank row either way")

    let unnamed = VehicleMessage.list([item("1", "sideways", "Nonsense", "still shown")])
    expect(unnamed.count == 1,
           "a level this head does not know still shows the message, because an operator not "
           + "seeing what the vehicle said is worse than seeing it in the wrong colour")
    expect(unnamed[0].level == .normal, "taking the quietest level it draws")

    let sameSecond = VehicleMessage.list([
        item("23:23:26.733", "warning", "Warning", "PreArm: waiting for home"),
        item("23:23:26.914", "warning", "Warning", "PreArm: waiting for home"),
    ])
    expect(Set(sameSecond.map(\.id)).count == 2,
           "two identical texts in the same second are two rows, because the id keeps the "
           + "milliseconds the row does not show; truncating it collapsed them into one")
    let sameStamp = VehicleMessage.list([
        item("23:23:26.733", "normal", "Info", "ready", component: 1),
        item("23:23:26.733", "normal", "Info", "ready", component: 2),
    ])
    expect(Set(sameStamp.map(\.id)).count == 2,
           "and two components saying the same thing at the same instant are two rows as well")

    expect(VehicleMessage.worst(read) == .error, "the worst of a run is what the panel reports")
    expect(VehicleMessage.worst([]) == .normal, "silence is not a fault")
    expect(VehicleMessage.worst(Array(read.dropFirst())) == .warning,
           "without the error, a warning is the worst")
}

checkVehicleMessages()

func checkCameraModeRow() {
    func camera(mode: Int, text: String) -> CameraControl {
        CameraControl(["present": true as NSNumber, "hasModes": true as NSNumber,
                       "mode": mode as NSNumber, "modeText": text,
                       "canChangeMode": true as NSNumber])
    }
    expect(camera(mode: 0, text: "Photo").offersModePicker,
           "a camera in photo mode gets the segmented picker, which can represent it")
    expect(camera(mode: 1, text: "Video").offersModePicker,
           "and so does video")
    expect(camera(mode: 2, text: "Survey").offersModePicker == false,
           "but SURVEY does not. video.rs spells three modes -- Photo, Video and Survey -- and the "
           + "picker carries exactly two tags, so a camera in survey matched NEITHER and SwiftUI "
           + "drew the segmented control with nothing selected: a blank where the operator looks "
           + "to see what the camera is doing. The row now draws the core's own modeText instead. "
           + "A third TAG would have been wrong -- there is no setCameraModeSurvey to invoke, so "
           + "it would offer a switch the head cannot perform")
    expect(camera(mode: -1, text: "Not set").offersModePicker == false,
           "and an undefined mode is the same case: the core already spells it \"Not set\", which "
           + "is a better answer than an empty segmented control")
    expect(camera(mode: 2, text: "Survey").modeText, "Survey",
           "and the word drawn is the core's, not one this head invents for a mode it cannot set")
}

func checkSetupBlocked() {
    func made(_ name: String, openable: Bool, reason: String?) -> [String: Any] {
        var json: [String: Any] = ["name": name, "className": name + "Component",
                                   "openable": openable as NSNumber]
        if let reason { json["blockedReason"] = reason }
        return json
    }
    let listed = VehicleComponentInfo.list([made("Sensors", openable: false, reason: "armed"),
                                            made("Safety", openable: true, reason: nil)])
    expect(VehicleComponentInfo.blockedSentence(for: "Sensors", in: listed) ?? "",
           "Disabled while the vehicle is armed",
           "a component the CORE says is not openable disables its page and says why, in the "
           + "original's own words. SetupPage.qml sets enabled: !_disableDueToArmed && "
           + "!_disableDueToFlying and prints \"Disabled while the vehicle is armed\"; this head "
           + "read neither openable nor blockedReason -- its only blockedReason decode is in "
           + "MissionItemModel, a different view -- so an ARMED vehicle still offered live "
           + "calibration Start buttons where QGC dims the page. Offering to start an accel "
           + "calibration on an armed aircraft is the reason that gate exists")
    expect(VehicleComponentInfo.blockedSentence(for: "Safety", in: listed) == nil,
           "and a component beside it that is openable is untouched, so the gate is per component "
           + "rather than per vehicle")
    expect(VehicleComponentInfo.blockedSentence(for: "Nothing", in: listed) == nil,
           "and a page with no component behind it is not blocked by a component that is missing")
    let wordless = VehicleComponentInfo.list([made("Sensors", openable: false, reason: nil)])
    expect(VehicleComponentInfo.blockedSentence(for: "Sensors", in: wordless) ?? "", "Disabled",
           "and if the core ever says not-openable without a word for why, the page still closes. "
           + "Failing OPEN on a safety gate would offer the control the core just refused, so the "
           + "least this can say is that it is disabled")
}

func checkModeRequest() {
    expect(FlightModes.requestResolved(requested: "Hold", askedFrom: "Position",
                                       now: "Position") == false,
           "a mode request stays pending while the vehicle has not moved, which is what the "
           + "header's spinner is for")
    expect(FlightModes.requestResolved(requested: "Hold", askedFrom: "Position", now: "Hold"),
           "and it resolves when the vehicle arrives")
    expect(FlightModes.requestResolved(requested: "Hold", askedFrom: "Position", now: "Return"),
           "and it ALSO resolves when the vehicle moves somewhere else, which is the case that "
           + "was missing. The old rule cleared only on requestedMode == state.mode, so a request "
           + "the vehicle REFUSED or a mode a failsafe overrode left the header spinning forever "
           + "-- and the header draws requestedMode INSTEAD of the real mode, so an aircraft that "
           + "dropped into Return on failsafe would still show the operator's stale ask. Hiding a "
           + "safety-relevant mode change behind a spinner is worse than showing no spinner")
    expect(FlightModes.requestResolved(requested: "", askedFrom: "Position", now: "Return") == false,
           "and with nothing pending there is nothing to resolve")
}

func checkChecklistReset() {
    let flying = FlyState(["connected": true as NSNumber, "armed": false as NSNumber,
                           "kind": "ready", "line": "Ready to fly"])
    expect(Preflight.ticksSurvive(flying, flying),
           "ticks survive an ordinary poll of the same connected vehicle, which is every reload")
    expect(Preflight.ticksSurvive(FlyState.none, flying),
           "and they survive a vehicle ARRIVING -- the checklist's first group is the one an "
           + "operator works through before anything is powered up, so ticks made with nothing "
           + "connected are about the aircraft that is now here")
    expect(Preflight.ticksSurvive(flying, FlyState.none) == false,
           "but a tick does NOT survive the vehicle going away. A tick is an assertion the "
           + "operator made about a SPECIFIC aircraft -- props clear, hatch closed -- and carrying "
           + "it onto the next one presents somebody else's inspection as this aircraft's. "
           + "PreFlightCheckList.qml holds `property var vehicleCopy: globals.activeVehicle` and "
           + "calls model.reset() in onVehicleCopyChanged, so the original clears them and this "
           + "head cleared them only from the manual Reset button. LIMIT, stated rather than "
           + "hidden: this head tracks presence and not identity, so it cannot see a switch "
           + "between two SIMULTANEOUSLY connected vehicles -- view.vehicles is unbuilt, so that "
           + "state cannot arise here yet, and a disconnect is the only route between aircraft")
}

func checkSetupCache() {
    expect(SettingsSection.worthRemembering([]) == false,
           "an EMPTY setup page is never remembered. view.setup(<page>) answers sections:0 until a "
           + "vehicle's parameters have loaded -- measured at 0 on this rig with nothing connected "
           + "-- and the store memoised that empty answer per page. The only line that clears the "
           + "cache runs after writing a CONTROL, so an empty page could never be refilled: there "
           + "is no control on it to write. Opening Vehicle Setup before the vehicle answered left "
           + "that page blank for the rest of the app's life")
    expect(SettingsSection.worthRemembering(
        [SettingsSection(title: "Safety", group: "", path: "", note: "", subsections: [])]) == true,
           "and a page that came back with real sections IS remembered, so the cache still does "
           + "the job it was added for")
}

func checkEditableFields() {
    expect(Measure.committed("75", showing: 75.4, decimals: 0) == nil,
           "an altitude field the operator FOCUSED AND LEFT WITHOUT TYPING must not write. The "
           + "field renders 75.4 as \"75\" at zero decimals, and the old guard compared the typed "
           + "number against the FULL-PRECISION value -- so 75.0 != 75.4 was true and merely "
           + "clicking into the field and clicking away committed 75, silently truncating the "
           + "plan's altitude. The guard meant to suppress a no-op write instead GUARANTEED one "
           + "for every value carrying a fraction")
    expect(Measure.committed("80", showing: 75.4, decimals: 0) == 80.0,
           "and a real edit still commits")
    expect(Measure.committed("75.4", showing: 75.4, decimals: 0) == nil,
           "and typing the value back in full precision is not a change either, even though the "
           + "text differs from what was shown")
    expect(Measure.committed("", showing: 75.4, decimals: 0) == nil,
           "and an unparseable draft writes nothing rather than zero")
    expect(Measure.fieldText(nil, 0), "",
           "a field with no value shows nothing, not a zero the operator could commit")
    expect(Measure.committed("2.5", showing: nil, decimals: 1) == 2.5,
           "and a field that had no value accepts the first number typed into it")
}

func checkDetections() {
    func box(_ label: String, _ x: Double, _ y: Double, _ w: Double, _ h: Double,
             conf: Double? = 0.91) -> [String: Any] {
        var made: [String: Any] = ["label": label, "x": x as NSNumber, "y": y as NSNumber,
                                   "w": w as NSNumber, "h": h as NSNumber]
        if let conf { made["confidence"] = conf as NSNumber }
        return made
    }
    func placed(_ picture: PaintedPicture?, _ json: [String: Any]) -> String {
        guard let picture, let decoded = DetectionBox(json) else { return "none" }
        return rect(picture.place(decoded))
    }
    func rect(_ picture: PaintedPicture?) -> String {
        guard let picture else { return "none" }
        func round(_ value: Double) -> String { String(format: "%.1f", value) }
        return "\(round(picture.x)),\(round(picture.y)) \(round(picture.width))x\(round(picture.height))"
    }

    let read = Detections(["available": true as NSNumber, "stale": false as NSNumber,
                           "boxes": [box("car", 0.1, 0.2, 0.3, 0.4),
                                     box("person", 0.5, 0.5, 0.2, 0.2, conf: nil)],
                           "error": ""])
    expect(read.boxes.count == 2, "every box the core sends is drawn")
    expect(read.boxes[0].caption, "car 91%", "a box says what it is and how sure it is")
    expect(read.boxes[1].caption, "person",
           "and one with no confidence says only what it is; the QML wrote \"person NaN%\"")
    expect(read.draws, "a fresh frame with boxes is drawn")

    let asCoreServes: [String: Any] = ["x": 0.1 as NSNumber, "y": 0.2 as NSNumber,
                                       "w": 0.3 as NSNumber, "h": 0.4 as NSNumber,
                                       "label": "car", "confidence": 0.91 as NSNumber,
                                       "target": true as NSNumber]
    expect(DetectionBox(asCoreServes)?.caption ?? "", "car 91%",
           "and the key is the one the CORE spells, copied from the shape detections.rs pins in "
           + "its own test. The core RENAMES the upstream \"conf\" to \"confidence\" on the way "
           + "out and this head went on reading the pre-rename name, so confidence was nil on "
           + "EVERY box and every detection drawn over the video was captioned \"car\" with no "
           + "percentage -- the number an operator uses to judge whether to trust the box. The "
           + "QML original drew it, so this was a regression, and the old test passed throughout "
           + "because its own fixture spelled the key the same wrong way the decoder did. A "
           + "hand-written fixture can only ever agree with the decoder; this one is copied from "
           + "the producer instead")
    expect(DetectionBox(["label": "car", "x": 0.1 as NSNumber, "y": 0.2 as NSNumber,
                         "w": 0.3 as NSNumber, "h": 0.4 as NSNumber,
                         "conf": 0.91 as NSNumber])?.caption ?? "", "car",
           "and the pre-rename spelling is pinned as NOT the key, so pointing the decoder back at "
           + "\"conf\" goes red here rather than silently dropping every percentage again")

    expect(Detections(["available": true as NSNumber, "stale": true as NSNumber,
                       "boxes": [box("car", 0, 0, 1, 1)]]).draws == false,
           "a stale frame is not drawn, even carrying boxes, because a box a second old is "
           + "somewhere the object has already left")
    expect(Detections(["available": false as NSNumber, "stale": false as NSNumber,
                       "boxes": [box("car", 0, 0, 1, 1)]]).draws == false,
           "and nothing is drawn for a camera the core is not watching")
    expect(Detections.none.draws == false, "the resting state draws nothing")

    expect(Detections(["available": true as NSNumber, "stale": false as NSNumber,
                       "boxes": [["label": "car", "x": 0 as NSNumber, "y": 0 as NSNumber,
                                  "w": 0 as NSNumber, "h": 0.5 as NSNumber],
                                 box("", 0, 0, 1, 1),
                                 box("nan", Double.nan, 0, 1, 1)]]).boxes.isEmpty,
           "a box with no width, no label, or a coordinate that is not a number is dropped "
           + "rather than drawn as a line or thrown at the corner")

    let pane = (width: 320.0, height: 180.0)
    let wide = SourceSize(["width": 1920 as NSNumber, "height": 1080 as NSNumber])
    expect(rect(wide?.painted(inWidth: pane.width, height: pane.height)), "0.0,0.0 320.0x180.0",
           "a source of the pane's own ratio fills it exactly")

    let squarish = SourceSize(["width": 640 as NSNumber, "height": 480 as NSNumber])
    let picture = squarish?.painted(inWidth: pane.width, height: pane.height)
    expect(rect(picture), "40.0,0.0 240.0x180.0",
           "a 4:3 source in a 16:9 pane is pillarboxed with a 40 point bar down each side")

    expect(placed(picture, box("all", 0, 0, 1, 1)), "40.0,0.0 240.0x180.0",
           "a box covering the whole frame covers the painted picture and not the bars; "
           + "normalising to the pane instead would put 40 points of it in black")
    expect(placed(picture, box("half", 0.5, 0.5, 0.25, 0.25)), "160.0,90.0 60.0x45.0",
           "and a box at the middle of the frame is at the middle of the picture")

    let tall = SourceSize(["width": 480 as NSNumber, "height": 640 as NSNumber])
    expect(rect(tall?.painted(inWidth: pane.width, height: pane.height)), "92.5,0.0 135.0x180.0",
           "a portrait source is bounded by the pane's height, not its width")

    expect(SourceSize(["width": 0 as NSNumber, "height": 480 as NSNumber]) == nil,
           "a source with no width is no source at all")
    expect(SourceSize(nil) == nil,
           "and the core sends null until a frame is decoding, which is why the head draws "
           + "nothing rather than guessing the pane is the picture")
    expect(rect(wide?.painted(inWidth: 0, height: 180)), "none",
           "a pane that has not been laid out yet has no picture in it either")
}

checkDetections()
checkEditableFields()
checkSetupCache()
checkChecklistReset()
checkModeRequest()
checkSetupBlocked()
checkCameraModeRow()

final class ProbeStub: Probeable {
    static let probeID = "stub"
    private(set) var invoked: [String] = []
    func probeState() -> [String: Any] { ["count": 2, "where": "the real store"] }
    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        invoked.append(action)
        return ["ok": true]
    }
}

func checkReadOnlyProbe() {
    let store = ProbeStub()
    let reader = ReadOnlyProbe(store, as: "flyMission")

    expect((reader.probeState()["where"] as? String) ?? "", "the real store",
           "a read-only handle shows the store's own state, which is the whole point of having it")
    expect((reader.probeState()["count"] as? Int) == 2, "every field, not a chosen few")

    let refused = reader.probeInvoke(action: "upload", args: [:])
    expect((refused["ok"] as? Bool) ?? true == false, "and refuses an action rather than running it")
    expect((refused["error"] as? String) ?? "", "flyMission is read-only",
           "naming itself, so a refusal in a log says which handle was used")
    expect(store.invoked.joined(separator: ","), "",
           "the refusal is a refusal, not a report: the plan store never saw the upload, which is "
           + "why the Fly view's copy is registered through this and not directly")
}

checkReadOnlyProbe()





func checkCameraControlMatchesTheCore() {
    let recording = CameraControl([
        "present": true as NSNumber, "title": "ZR30", "modeText": "Video",
        "stateText": "Recording 00:01:15", "clockText": "00:01:15",
        "storageText": "12 GB", "shotsText": "00042", "batteryText": "80%",
        "isRecording": true as NSNumber,
        "canPhoto": false as NSNumber, "canRecord": true as NSNumber,
    ])
    expect(recording.present, "the camera the core describes is present")
    expect(recording.title, "ZR30", "and reads by its model name")
    expect(recording.modeText, "Video", "in the mode the core named")
    expect(recording.stateText, "Recording 00:01:15", "with the core's sentence, not one built here")
    expect(recording.shotsText, "00042", "and the core's shot counter, zeros and all")
    expect(recording.batteryText, "80%", "and its battery reading")

    expect(!recording.canPhoto,
           "a camera in video mode cannot take a photo, so the Fly view offers no shutter; this "
           + "is the core's canPhoto and not this head re-reading the mode")
    expect(recording.canRecord, "but it can record, so the record button is offered")
    expect(!recording.offersShutter && recording.offersRecord,
           "and one property decides that for both the store's guard and the view's condition, "
           + "so a press cannot reach a camera the core says cannot take it")

    let photographing = CameraControl(["present": true as NSNumber, "title": "Camera",
                                       "canPhoto": true as NSNumber,
                                       "canRecord": false as NSNumber])
    expect(photographing.offersShutter,
           "a camera the core says can take a photo offers its shutter -- a guard shown only to "
           + "refuse has never been shown to let anything through")
    expect(!photographing.offersRecord, "and offers no record button it was not given")
    expect(recording.isRecording, "and it says Stop rather than Record while running")

    let absent = CameraControl(["present": false as NSNumber, "title": "Camera",
                                "canRecord": false as NSNumber, "canPhoto": false as NSNumber])
    expect(!absent.present, "with no camera the core reports none")
    expect(absent.title, "Camera", "under a plain name rather than a blank row")
    expect(!absent.offersShutter && !absent.offersRecord,
           "so neither control is reachable at all")

    let inconsistent = CameraControl(["present": false as NSNumber,
                                      "canPhoto": true as NSNumber,
                                      "canRecord": true as NSNumber])
    expect(!inconsistent.offersShutter && !inconsistent.offersRecord,
           "a camera that is not there offers nothing to press even if the flags say otherwise")
    expect(!absent.canRecord && !absent.canPhoto,
           "and neither button is offered, which is what keeps a shutter off a vehicle that has "
           + "no camera to shoot with")
}

checkCameraControlMatchesTheCore()

func checkRestartNoticeReachesTheRow() {
    func control(_ reboot: Bool, label: String = "Application font size") -> SettingsControl {
        SettingsControl(["path": "settings.appSettings.appFontPointSize",
                         "name": "appFontPointSize", "label": label, "control": "number",
                         "valueString": "13", "units": "pt",
                         "restartNotices": reboot ? ["Application restart required after change"]
                                                  : []])!
    }

    expect(control(true).restartNotice, "Application restart required after change",
           "the core names WHICH restart, and this head used to say the plainer "
           + "\"Restart required after a change\" because view.control once folded "
           + "vehicleRebootRequired and qgcRebootRequired into one flag. It does not any more, "
           + "and an operator told only \"restart\" does not know whether to relaunch the "
           + "application or reboot the aircraft -- two very different acts, one of them on a "
           + "machine that may be armed. MEASURED on settings.appSettings.qLocaleLanguage")
    expect(control(false).restartNotice, "",
           "and a setting that takes effect at once says nothing extra -- measured empty on "
           + "appFontPointSize, indoorPalette and savePath")

    expect(SettingsControl(["path": "p", "name": "n", "label": "L", "control": "number",
                            "valueString": "1", "units": "",
                            "restartNotices": ["Vehicle reboot required after change",
                                               "Application restart required after change"]])!
                .restartNotice,
           "Vehicle reboot required after change \u{00B7} Application restart required after change",
           "and a fact needing BOTH names both rather than collapsing to one, which is the case "
           + "the old single flag could not express at all")

    expect(control(true).rowDescription(label: "Application font size"),
           "appFontPointSize \u{00B7} Application restart required after change",
           "the row carries the notice beside the setting's own name, on the line an operator is "
           + "already reading; it used to be decoded and then dropped on the floor")
    expect(control(false).rowDescription(label: "Application font size"), "appFontPointSize",
           "and is otherwise the name alone")
    expect(control(true, label: "").rowDescription(label: ""),
           "Application restart required after change",
           "a control with no label puts its name in the title, so the description is the notice "
           + "by itself rather than the name written twice")
}

checkRestartNoticeReachesTheRow()

func checkPreflight() {
    func check(_ name: String, _ verdict: String, _ blocked: Bool) -> [String: Any] {
        ["name": name, "prompt": "P", "verdict": verdict, "reason": "R",
         "blocked": blocked as NSNumber]
    }
    let json: [Any] = [
        ["name": "Before you power up",
         "checks": [check("Hardware", "manual", false),
                    check("Battery", "failing", true),
                    check("GPS", "overridable", false)]],
        ["name": "Arm the vehicle here", "checks": [check("Motors", "passing", false)]],
    ]

    let groups = Preflight.groups(json)
    expect(groups.count == 2, "each group the core describes is carried across")
    expect(groups.map(\.name).joined(separator: "|"), "Before you power up|Arm the vehicle here",
           "in the order the core listed them, because it is a sequence the operator works down")
    expect(Preflight.total(groups) == 4, "and every check in them is counted")

    let battery = groups[0].checks[1]
    expect(battery.blocked, "a failing check blocks, so it cannot be ticked off")
    expect(battery.reason, "R", "and shows the core's reason rather than the prompt")

    expect(!groups[0].checks[2].blocked,
           "an overridable check does not block, which is the whole difference between it and "
           + "failing, and reading only two verdicts would have made a soft GPS warning unclearable")

    let motors = groups[1].checks[0]
    expect(motors.alreadyMet,
           "the core has already verified a passing check, so the operator is not asked to "
           + "confirm by hand what a machine has measured")
    expect(!motors.tickable, "there is nothing left to tick on it")
    expect(motors.symbol(ticked: []), "checkmark.circle.fill",
           "and it reads as met without a tick, the way QGC passes a check with no manual text")

    let gps = groups[0].checks[2]
    expect(gps.warns, "an overridable check is a real problem the operator may fly past")
    expect(gps.tickable, "so it is the operator's to clear")
    expect(gps.symbol(ticked: []), "exclamationmark.triangle.fill",
           "and it must not look like a routine item; it did, because the head read only blocked")
    expect(gps.hint, "R Check it off to fly anyway.",
           "the hint says what is wrong and that flying anyway is allowed")
    expect(gps.symbol(ticked: ["GPS"]), "checkmark.circle.fill", "once cleared it reads as met")

    let hardware = groups[0].checks[0]
    expect(hardware.symbol(ticked: []), "circle", "a manual check is a plain empty circle")
    expect(hardware.tickable, "and is the operator's to confirm")
    expect(hardware.hint, "P", "whose hint is the core's prompt")

    expect(battery.symbol(ticked: []), "exclamationmark.octagon.fill", "a failing check stops the list")
    expect(!battery.tickable, "and cannot be ticked past")
    expect(battery.symbol(ticked: ["Battery"]), "exclamationmark.octagon.fill",
           "not even by a stale tick left in the set from before it started failing")

    groups.flatMap(\.checks).forEach { check in
        expect(check.blocked == check.blocks,
               "the core sets blocked on exactly the failing verdict, so \(check.name) agrees; "
               + "the head drives display from the verdict and this is what pins the two together")
    }

    expect(Preflight.progress(groups, ticked: []), "1 of 4 checked",
           "an untouched list already counts what the core has passed; it used to say 0 of 4 and "
           + "made the operator tick off a check that had already been measured")
    expect(!Preflight.ready(groups, ticked: []), "and is not ready while three are outstanding")
    let every: Set<String> = ["Hardware", "Battery", "GPS", "Motors"]
    expect(!Preflight.ready(groups, ticked: every),
           "ticking every name is still not ready, because a failing check cannot be ticked past")
    let flyable: Set<String> = ["Hardware", "GPS"]
    expect(Preflight.progress(groups, ticked: flyable), "3 of 4 checked",
           "the two the operator answered plus the one the core passed")
    let strange = PreflightCheck(check("Payload", "invented", false))
    expect(strange?.verdict == .manual,
           "a verdict this head does not know is decoded as one the operator must look at")
    expect(strange?.alreadyMet == false,
           "so a verdict the core adds later cannot clear a safety check on its own; asserting on "
           + "the constant instead of the decode let a default of .passing through the break")
    expect(strange?.tickable == true, "and the operator can still answer it")

    expect(PreflightCheck(["name": "", "verdict": "manual"]) == nil,
           "a check with no name is dropped, because the name is the identity a tick is stored under")
    expect(PreflightCheck(["name": "GPS"]) == nil,
           "and one with no verdict is dropped rather than read as passing")
    expect(Preflight.groups(nil).isEmpty, "no answer is no checklist")
}
checkPreflight()

func checkVehicleWarning() {
    let live: [Any] = [
        ["id": "prearm", "text": "PreArm: GPS 1: not healthy",
         "detail": "The vehicle has failed a pre-arm check. In order to arm the vehicle, resolve the failure."],
        ["id": "noGpsLock", "text": "No GPS lock for vehicle",
         "detail": "This vehicle needs a position fix before it will arm."],
    ]
    let listed = VehicleWarning.list(live)
    expect(listed.count == 2, "each warning the core raises is carried across")
    expect(listed.map(\.id).joined(separator: "|"), "prearm|noGpsLock",
           "in the order the core listed them")
    expect(listed[0].text, "PreArm: GPS 1: not healthy",
           "a prearm refusal is shown as the vehicle worded it")
    expect(listed[0].detail.contains("resolve the failure"),
           "with the core's explanation of what to do about it, which this banner could not say before")

    expect(VehicleWarning.list([]).isEmpty, "no warnings is an empty banner, not an empty line")
    expect(VehicleWarning.list(nil).isEmpty, "and no answer is the same")

    expect(VehicleWarning(["id": "prearm"]) == nil,
           "a warning with no text is dropped rather than drawn as a blank row")
    expect(VehicleWarning(["text": "something is wrong"]) == nil,
           "and one with no id is dropped, because the id is what the row is keyed by")
    expect(VehicleWarning(["id": "noGpsLock", "text": "No GPS lock for vehicle"])?.detail ?? "x", "",
           "a warning the core sends without detail shows its headline alone")

    expect(VehicleWarning.list([["id": "prearm", "text": "PreArm: GPS 1: not healthy"]])
        .map(\.text).joined(separator: "|"), "PreArm: GPS 1: not healthy",
           "when a prearm warning is raised the banner shows it, and the arming blocker repeats "
           + "that same sentence, which is why the blocker is only drawn when no warning is")
}
checkVehicleWarning()

func checkInstrumentValues() {
    let items: [Any] = [
        ["id": "vehicle/altitudeRelative", "group": "vehicle", "name": "altitudeRelative",
         "label": "Alt (Rel)", "value": "-0.0", "units": "m", "missing": false as NSNumber],
        ["id": "batteries.0/voltage", "group": "batteries.0", "name": "voltage",
         "label": "Voltage", "value": "12.60", "units": "V", "missing": false as NSNumber],
        ["id": "vehicle/notAFactAtAll", "group": "vehicle", "name": "notAFactAtAll",
         "label": "Not A Fact At All", "value": "\u{2014}", "units": "", "missing": true as NSNumber],
    ]
    let read = InstrumentValue.list(items)
    expect(read.count == 3, "every reading the core resolves is carried across")
    expect(read[0].label, "Alt (Rel)",
           "the label is the core's, taken from the fact's own description")
    expect(read[1].units, "V",
           "and the unit is the core's display form, so no head maps a lowercase v itself")
    expect(read[1].id, "batteries.0/voltage",
           "a reading from another group keeps its group in the id, which is what the slot is keyed by")

    let absent = read[2]
    expect(absent.missing, "a fact the vehicle does not report is marked missing")
    expect(absent.value, "\u{2014}", "and shows a dash")
    expect(absent.units, "", "with no unit, because a unit beside a dash reads as a real measurement")

    expect(InstrumentValue.list(nil).isEmpty, "no answer is no readings")
    expect(InstrumentValue(["id": "vehicle/x"]) == nil,
           "an item with no name is dropped rather than drawn as a blank slot")
    expect(InstrumentValue(["name": "heading"]) == nil,
           "and one with no id is dropped, because the id is what the slot is keyed by")
}
checkInstrumentValues()

func checkInstrumentGroups() {
    let read: [(group: String, json: [String: Any])] = [
        ("", ["facts": [["name": "heading", "shortDescription": "Heading"],
                        ["name": "airSpeedSetpoint", "shortDescription": ""]]]),
        ("rpm", ["kind": "value"]),
        ("gps", ["facts": [["name": "lat", "shortDescription": "Latitude"]]]),
        ("batteries.0", ["facts": [["name": "voltage", "shortDescription": "Voltage"]]]),
    ]

    let groups = InstrumentGroup.assemble(read, label: { "<\($0)>" })
    expect(groups.map(\.title).joined(separator: ","), "Vehicle,<gps>,Battery 1",
           "a child carrying no facts is dropped, so rpm never reaches the picker, and a group "
           + "the head does not name itself is labelled by the core")
    expect(groups[0].facts.map(\.label).joined(separator: ","), "Heading,<airSpeedSetpoint>",
           "a described fact keeps the vehicle's own description, and only one without falls "
           + "back to the core's label for its name")
    expect(groups[0].facts.map(\.name).joined(separator: ","), "heading,airSpeedSetpoint",
           "while the name stays exactly as the vehicle reports it")

    expect(InstrumentGroup.title(for: "", label: { $0 }), "Vehicle",
           "the vehicle's own facts are titled for it without asking the core")
    expect(InstrumentGroup.title(for: "batteries.0", label: { $0 }), "Battery 1",
           "an indexed battery reads as a battery, not as a path, and also without asking")
    expect(InstrumentGroup.title(for: "estimatorStatus", label: { "<\($0)>" }), "<estimatorStatus>",
           "while any other group is named by the core")

    expect(InstrumentGroup.assemble([("gps", ["facts": [["shortDescription": "no name"]]])],
                                    label: { $0 }).isEmpty,
           "a fact with no name is not offered, so an empty group is dropped entirely")

    let aliased: [(group: String, json: [String: Any])] = [
        ("", ["facts": [["name": "heading"]]]),
        ("vehicle", ["facts": [["name": "heading"]]]),
        ("gps", ["facts": [["name": "lat"], ["name": "lon"]]]),
        ("gps2", ["facts": [["name": "lat"], ["name": "lon"]]]),
    ]
    expect(InstrumentGroup.assemble(aliased, label: { "<\($0)>" }).map(\.title)
        .joined(separator: ","), "Vehicle,<gps>,<gps2>",
           "the vehicle lists itself among its children, so that one alias goes, and two GPS "
           + "units share a schema so both stay and both are named by the core")
}

checkInstrumentGroups()

func checkInstrumentStorage() {
    expect(InstrumentSelection.vehicle("heading").stored, "/heading",
           "a vehicle fact stores with an empty group")
    expect(InstrumentSelection(group: "batteries.0", factName: "voltage").stored,
           "batteries.0/voltage", "a group keeps the dots in its own name")

    expect(InstrumentSelection.decode("wind/speed")?.group ?? "?", "wind", "and reads back")
    expect(InstrumentSelection.decode("wind/speed")?.factName ?? "?", "speed", "both halves")
    expect(InstrumentSelection.decode("/heading")?.group ?? "?", "",
           "an empty group survives the round trip")
    expect(InstrumentSelection.decode("batteries.0/voltage")?.factName ?? "?", "voltage",
           "splitting on the first slash keeps a dotted group intact")
    expect(InstrumentSelection.decode("noslash") == nil, "a stored value with no slash is not a selection")
    expect(InstrumentSelection.decode("wind/") == nil, "nor is one naming no fact")

    expect(InstrumentSelection.restore(nil).count == 6, "nothing stored falls back to the defaults")
    expect(InstrumentSelection.restore([]).count == 6, "so does an empty list")
    expect(InstrumentSelection.restore(["junk", "also junk"]).count == 6,
           "so does a stored list that decodes to nothing, rather than an empty bar")
    expect(InstrumentSelection.restore(["wind/speed"]).count == 1,
           "one good entry is honoured on its own")
    expect(InstrumentSelection.restore(["wind/speed", "junk"]).map(\.stored).joined(separator: ","),
           "wind/speed", "and a partly damaged list keeps what still reads")

    let used = [InstrumentSelection.vehicle("altitudeRelative"), .vehicle("groundSpeed")]
    expect(InstrumentSelection.firstUnused(in: used).factName, "climbRate",
           "a new slot takes the first default not already shown")
    expect(InstrumentSelection.firstUnused(in: InstrumentSelection.defaults).factName,
           "altitudeRelative", "and falls back to the first once every default is used")
    expect(InstrumentSelection.firstUnused(in: []).factName, "altitudeRelative",
           "an empty bar starts from the first")
}

checkInstrumentStorage()

checkGuidedOffers()
checkViewContract()
checkMotorTest()
checkMapClick()
checkMapCentre()
checkPolygonEdit()
checkMessageRate()
checkMenuPlacement()
checkCentreNotes()
checkPlanViewState()
checkMapFollow()
checkMapScale()
checkTerrainDownload()
checkMyLocation()
checkFlyOverlays()
checkSetupPages()
checkRemoteSupport()

if failures == 0 {
    print("all Swift checks passed")
    exit(0)
}
FileHandle.standardError.write("\(failures) check(s) failed\n".data(using: .utf8)!)
exit(1)

func checkLaunchPosition() {
    let absent = LaunchPosition(home: ["valid": false], item: [:])
    expect(absent.editable, "no vehicle home means the plan owns the launch position")
    expect(absent.positionText == "Not set", "an unplaced launch position says so")

    let onVehicle = LaunchPosition(home: ["valid": true],
                                   item: ["coordinate": ["latitude": -35.36 as NSNumber,
                                                         "longitude": 149.16 as NSNumber],
                                          "altitude": 583.0 as NSNumber,
                                          "altitudeUnits": "m", "altitudeText": "583 m"])
    expect(!onVehicle.editable, "the vehicle's own home position wins over the plan's")
    expect(onVehicle.altitudeText, "583 m",
           "the launch height is the string the CORE spelled on the plan's first item, which is "
           + "where its place already came from. This head took the item's raw altitude and "
           + "units and re-spelled them through its own copy of the core's formatter -- the copy "
           + "that was missing settled() and drew \"-0.0 m\" for a vehicle on the ground")
    expect(onVehicle.positionText == "-35.360000, 149.160000", "a placed launch position reads out")

    let feet = LaunchPosition(home: ["valid": false],
                              item: ["altitude": 100.0 as NSNumber, "altitudeUnits": "ft",
                                     "altitudeText": "100 ft"])
    expect(feet.units, "ft",
           "the RAW number and its unit are kept as well, because this altitude is EDITED rather "
           + "than read: PlanWindow hands them to an AltitudeField with a commit. A spelled "
           + "string cannot be typed into. cmake caught that when swift-checks did not, because "
           + "PlanWindow.swift is not in the checks' compile list")
    expect(feet.altitudeText, "100 ft",
           "so an operator on feet is not told metres, and the head no longer needs to know "
           + "which unit the core chose")
    expect(LaunchPosition(home: ["valid": false], item: [:]).altitudeText, Measure.unreported,
           "an item the core spelled no altitude for reads as absent rather than as a zero")
    expect(feet.note.contains("ground height"),
           "and the note says the terrain fills that altitude in, which QGC does two seconds later")
}

func checkMissionStartSpeed() {
    let speed = ItemSpeed(json: ["available": true, "specifyFlightSpeed": false,
                                 "facts": [["name": "FlightSpeed", "value": 8.0, "units": "m/s"]]])
    let arduPilot = MissionVehicle(firmware: "ArduPilot", type: "Quadrotor",
                                   multiRotor: true, vtol: false, apmFirmware: true)
    let px4 = MissionVehicle(firmware: "PX4 Pro", type: "Quadrotor",
                             multiRotor: true, vtol: false, apmFirmware: false)
    let vtol = MissionVehicle(firmware: "PX4 Pro", type: "VTOL",
                              multiRotor: false, vtol: true, apmFirmware: false)

    expect(speed.shown(missionStart: false, vehicle: arduPilot),
           "a waypoint offers its own speed whatever the firmware is")
    expect(!speed.shown(missionStart: true, vehicle: arduPilot),
           "but mission start does not on ArduPilot, which is what QGC hides")
    expect(speed.shown(missionStart: true, vehicle: px4), "on PX4 it does")
    expect(!speed.shown(missionStart: true, vehicle: vtol), "and never on a VTOL")

    let absent = ItemSpeed.unavailable
    expect(!absent.shown(missionStart: false, vehicle: px4),
           "an item with no speed section offers nothing regardless")
}

func checkLaunchAltitudeIsNotListedTwice() {
    let facts = ItemFact.owned([
        ["property": "plannedHomePositionAltitude", "name": "PlannedHomePositionAltitude",
         "units": "m", "valueString": "0.0"],
        ["property": "holdTime", "name": "Hold", "units": "secs", "valueString": "0"],
    ], label: { "<\($0)>" })
    expect(facts.count == 1, "the launch altitude is left out of the generic settings list")
    expect(facts.first?.title ?? "", "<Hold>", "the item's own facts are still listed")
}

func checkMotorTest() {
    let apm = MotorTest(reportedCount: 4, letterIndices: true, connected: true, armed: false)
    expect(apm.names.joined(separator: ","), "A,B,C,D",
           "ArduPilot names its motors by letter, which is what its own page shows")
    expect(apm.countWarning, "", "a vehicle that reported four motors needs no warning")

    let px4 = MotorTest(reportedCount: 6, letterIndices: false, connected: true, armed: false)
    expect(px4.names.joined(separator: ","), "1,2,3,4,5,6", "PX4 numbers them instead")

    let unknown = MotorTest(reportedCount: MotorTest.unknownCount, letterIndices: false,
                            connected: true, armed: false)
    expect(unknown.motors == MotorTest.fallbackMotors,
           "a vehicle that never said how many motors it has gets eight buttons, as QGC does")
    expect(!unknown.countWarning.isEmpty, "and is told why there are eight")
    expect(MotorTest.disconnected.countWarning, "",
           "but with nothing connected there is no vehicle that has not said. The page is gated "
           + "by SetupPageBody so the screen never drew this, and Motors.swift put it in the "
           + "probe anyway -- an instrument reporting a sentence the window cannot show is the "
           + "reason a defect was nearly filed against it")

    expect(!apm.canTest(safetyOff: false), "nothing spins until the safety switch is on")
    expect(apm.canTest(safetyOff: true), "with it on and the vehicle disarmed a motor can be tested")
    expect(!MotorTest.disconnected.canTest(safetyOff: true),
           "and never with no vehicle to send it to")

    let armed = MotorTest(reportedCount: 4, letterIndices: true, connected: true, armed: true)
    expect(!armed.canTest(safetyOff: true), "nor while the vehicle is armed")
    expect(!armed.armedRefusal.isEmpty, "which the page says rather than just disabling the buttons")
    expect(apm.armedRefusal, "", "a disarmed vehicle is not scolded")

    expect(MotorTest.timeout(throttle: 0) == 0,
           "a motor asked for no throttle is stopped, not run for three seconds")
    expect(MotorTest.timeout(throttle: 20) == MotorTest.timeoutSeconds,
           "any other throttle runs for the timeout QGC uses")
    expect(MotorTest.clamp(140) == 100, "throttle cannot exceed full")
    expect(MotorTest.clamp(-5) == 0, "nor go below nothing")

    expect(MotorTest.safetyText(false).contains("propellers"),
           "the switch off says to remove the propellers first")
    expect(MotorTest.safetyText(true).contains("turn"),
           "and switched on says the motors will turn")
}

func checkRemoteSupport() {
    let ready = RemoteSupport(host: "support.ardupilot.org:1234", forwarding: false)
    expect(ready.canConnect, "a host and no forwarding yet means connect is offered")
    expect(ready.status.contains("Nothing"), "and the page says nothing is being forwarded")

    expect(!ready.canStop, "with nothing being forwarded there is nothing to stop")
    expect(ready.actionTitle, "Connect", "so the button offers to start")

    let running = RemoteSupport(host: "support.ardupilot.org:1234", forwarding: true)
    expect(!running.canConnect, "forwarding cannot be started twice")
    expect(running.canStop,
           "but it can be stopped: LinkManager latched this flag on with no way to clear it, and "
           + "these tests pinned that as a fact about QGC rather than as the defect it was")
    expect(running.actionTitle, "Stop", "so the same button offers the way out")
    expect(!running.status.contains("restart"),
           "and the page no longer tells the operator to restart the app, which was the head "
           + "writing the latch down as though it were a design")
    expect(running.status, "MAVLink is being forwarded.", "it says what is happening, and stops there")

    expect(!RemoteSupport(host: "", forwarding: false).canConnect,
           "a button that would forward to nowhere is refused rather than offered")
    expect(!RemoteSupport.empty.canConnect, "same before the setting has been read")
}

func checkSetupPages() {
    expect(SetupPage.symbol(for: "Anything Unknown"), SetupPage.unknownSymbol,
           "and a page added to the core tomorrow draws as unknown rather than as something else")

    let decoded = SetupCatalogue.groups([
        ["title": "Vehicle", "pages": [["name": "Summary",
                                        "parameterSections": false as NSNumber]]],
        ["title": "Bespoke", "pages": [["name": "Safety",
                                        "parameterSections": true as NSNumber],
                                       ["name": "Sensors",
                                        "parameterSections": false as NSNumber]]],
        ["title": "Sections", "pages": [["name": "Flight Behavior",
                                         "parameterSections": true as NSNumber]]],
        ["title": "Empty", "pages": [["name": "Invented Page",
                                      "parameterSections": false as NSNumber]]],
    ])
    expect(SetupCatalogue.names(decoded).joined(separator: ","),
           "Summary,Safety,Sensors,Flight Behavior,Invented Page",
           "every page the core lists is decoded, drawable or not")

    let offered = SetupCatalogue.offered(decoded)
    expect(SetupCatalogue.names(offered).joined(separator: ","),
           "Summary,Safety,Sensors,Flight Behavior",
           "this head decides what it can draw: a name its content switch has a case for, or "
           + "anything with parameter sections. view.setup used to answer that with a native "
           + "flag, which had the core saying what a head is capable of -- and the sidebar went "
           + "blank the moment the core moved the flag")
    expect(SetupCatalogue.page("Invented Page", in: offered) == nil,
           "a page this head has no view for and no sections to fall back on is not offered, "
           + "because choosing it would open on nothing")
    expect(SetupCatalogue.page("Summary", in: offered) != nil,
           "Summary is offered like any other page. It used to reach the screen only as the "
           + "switch's fallback, so the moment the sidebar stopped listing it the operator had "
           + "no way back to the one page that says what still needs setting")
    expect(offered.map(\.title).joined(separator: ","), "Vehicle,Bespoke,Sections",
           "a group left with no pages at all goes with them, rather than sitting empty")
    expect(SetupCatalogue.page("Safety", in: offered)?.parameterSections == true,
           "a page still carries whether the core has parameter sections for it, which is what "
           + "the fallback draws from -- that is a fact about the DATA, not about this head")

    expect(SetupPage.bespoke.filter { SetupPage.glyphs[$0] == nil }.sorted()
        .joined(separator: ","), "",
           "every page the content switch draws has a glyph. These are two hand-written lists in "
           + "one file answering to different things -- glyphs to the core's page names, bespoke "
           + "to the switch -- and a page in the switch with no glyph draws the unknown icon, "
           + "which is exactly what that icon is for, so nobody would notice")
    expect(SetupPage.glyphs["Flight Behavior"] != nil && !SetupPage.bespoke.contains("Flight Behavior"),
           "the two lists are legitimately different, so this is a subset and not an equality: "
           + "Flight Behavior has a glyph and no bespoke view, and draws from parameter sections")

    expect(SetupPage.draws(SetupPageInfo(name: "Radio", parameterSections: false)),
           "a bespoke page draws with no sections at all")
    expect(SetupPage.draws(SetupPageInfo(name: "Anything", parameterSections: true)),
           "and a page this head never heard of draws when the core has sections for it")
    expect(!SetupPage.draws(SetupPageInfo(name: "Anything", parameterSections: false)),
           "with neither, it does not")

    expect(SetupCatalogue.groups(nil).isEmpty, "no answer is no pages")
    expect(SetupCatalogue.groups([["pages": []], ["title": ""]]).isEmpty,
           "a group with no title is dropped rather than drawn as a blank heading")

    let selection = PageSelection(owner: "test", pages: ["Summary"])
    selection.offer(["Summary", "Safety", "Tuning"])
    expect(selection.pages.joined(separator: ","), "Summary,Safety,Tuning",
           "the window navigates by the list the core gave it")
    selection.page = "Tuning"
    selection.offer([])
    expect(selection.pages.count == 3,
           "an empty read is a read that failed, not a window with no pages, so it is ignored")
    expect(selection.page, "Tuning", "and the page the operator was on is left alone")
    selection.offer(["Summary", "Safety"])
    expect(selection.page, "Summary",
           "but a page that stops existing -- Sensors on PX4 -- moves the operator somewhere real "
           + "rather than leaving the sidebar pointing at nothing")
}

func checkMapClick() {
    expect(MapClickAction.offered(in: MapClickState()).isEmpty,
           "with no vehicle the map commands nothing")
    var winged = MapClickState()
    winged.gotoLoiterRadius = 120
    expect(winged.gotoLoiterRadius == 120,
           "a goto carries the radius the core read from the operator's setting, not a zero this "
           + "head chose. A fixed wing cannot hover, so a goto has to say how wide to circle, and "
           + "every goto sent from here used to carry 0 -- a number on the wire nobody picked")
    expect(MapClickState().gotoLoiterRadius == 0,
           "and a vehicle that does not fly forward gets no radius, because the core answers 0 "
           + "for it rather than the head deciding when a radius applies")

    expect(MapClickAction.refusal(in: MapClickState()).contains("No vehicle"),
           "and says why rather than showing an empty menu")

    var grounded = MapClickState()
    grounded.connected = true
    grounded.roiSupported = true
    grounded.orbitSupported = true
    grounded.homeUsable = true
    var orbiting = MapClickState()
    orbiting.connected = true
    orbiting.flying = true
    orbiting.orbitSupported = true
    orbiting.homeUsable = true
    expect(!MapClickAction.offered(in: orbiting).contains(.orbit),
           "an orbit is NOT offered even where QGC offers one. guidedModeOrbit takes a radius "
           + "whose sign is the turn direction and an AMSL altitude, and this head asks the "
           + "operator for neither -- it sent (centre, 0, 0), where 0 AMSL is sea level. A "
           + "guided command to a flying aircraft carrying two invented numbers fails by doing "
           + "something, which is why Clear Mission is withheld too")

    expect(MapClickAction.offered(in: grounded) == [.setHome],
           "on the ground only Set home is offered, which is QGC's showSetHome with no flying gate")

    var flying = grounded
    flying.flying = true
    expect(MapClickAction.offered(in: flying) == [.goTo, .roi, .setHome, .setHeading],
           "in the air the guided commands join it, in QGC's own order -- minus the orbit, which "
           + "this head withholds until it can say how wide to circle and how high")

    var noOrbit = flying
    noOrbit.orbitSupported = false
    expect(!MapClickAction.offered(in: noOrbit).contains(.orbit),
           "a vehicle that does not support orbit is not offered it, which is this ArduCopter")

    var onMission = flying
    onMission.missionActive = true
    expect(!MapClickAction.offered(in: onMission).contains(.orbit),
           "nor is one already flying a mission")
    expect(MapClickAction.offered(in: onMission).contains(.goTo),
           "though it can still be sent somewhere")

    var noHome = flying
    noHome.homeUsable = false
    expect(!MapClickAction.offered(in: noHome).contains(.orbit),
           "orbit needs a home altitude to circle at, so without one it is withheld")

    var looking = flying
    looking.roiActive = true
    expect(MapClickAction.offered(in: looking).contains(.roi),
           "an aimed camera can be re-aimed by clicking somewhere else, which is what QGC's "
           + "showROI does \u{2014} it has no roiActive term. This head used to hide the option "
           + "and make the operator cancel first, losing the aim in between")
    expect(MapClickAction.offered(in: looking).contains(.cancelRoi),
           "and releasing it is offered alongside rather than instead. QGC keeps Cancel ROI on "
           + "the ROI marker's own drop panel; this map menu carries both, which is the union of "
           + "what QGC offers across its two menus")

    var noGps = flying
    noGps.gpsSensorPresent = false
    expect(MapClickAction.offered(in: noGps).contains(.setEstimatorOrigin),
           "a vehicle with no GPS sensor can be told where it is starting from")
    expect(!MapClickAction.offered(in: flying).contains(.setEstimatorOrigin),
           "one with GPS never is")

    expect(MapClickAction.setHome.needsConfirmation(in: flying),
           "every command off the map is confirmed")
    var guidedNoConfirm = flying
    guidedNoConfirm.inGotoMode = true
    guidedNoConfirm.confirmGotoInGuided = false
    expect(!MapClickAction.goTo.needsConfirmation(in: guidedNoConfirm),
           "except a fly-to while already in guided mode with the setting turned off, which is QGC's one exemption")
    guidedNoConfirm.confirmGotoInGuided = true
    expect(MapClickAction.goTo.needsConfirmation(in: guidedNoConfirm),
           "the setting on puts the confirmation back")
    var notGuided = flying
    notGuided.confirmGotoInGuided = false
    expect(MapClickAction.goTo.needsConfirmation(in: notGuided),
           "and the exemption does not apply outside guided mode")
    expect(MapClickAction.orbit.needsConfirmation(in: guidedNoConfirm),
           "the exemption is for fly-to alone, not for everything")

    expect(Set(MapClickAction.allCases.map(\.invokable)).count == MapClickAction.allCases.count,
           "no two commands send the same thing")
    expect(Set(MapClickAction.allCases.map(\.title)).count == MapClickAction.allCases.count,
           "and no two read alike")

    expect(MapClickState.homeUsable(GeoPoint(latitude: -35.36, longitude: 149.16), altitude: 584),
           "a home position with a real place and a real height can be orbited around")
    expect(!MapClickState.homeUsable(GeoPoint(latitude: -35.36, longitude: 149.16), altitude: nil),
           "the vehicle group serialises a coordinate without its valid flag, so validity is read off the numbers")
    expect(!MapClickState.homeUsable(GeoPoint(latitude: 0, longitude: 0), altitude: 0),
           "and null island is the home position ArduPilot sends before it knows one")
    expect(!MapClickState.homeUsable(nil, altitude: 584), "no home at all is not a home")
    expect(!MapClickState.homeUsable(GeoPoint(latitude: -35.36, longitude: 149.16), altitude: .nan),
           "nor is one at an unknown height")
}

func checkMapCentre() {
    let empty = MapCentreState()
    expect(!MapCentre.mission.enabled(in: empty), "a plan with no items has no mission to centre on")
    expect(!MapCentre.allItems.enabled(in: empty), "nor anything else")
    expect(!MapCentre.launch.enabled(in: empty), "nor a launch point")
    expect(!MapCentre.vehicle.enabled(in: empty), "nor a vehicle")
    expect(MapCentre.coordinates.enabled(in: empty),
           "but coordinates can always be typed, which is why QGC leaves that one ungated")

    var planned = MapCentreState()
    planned.missionPoints = [GeoPoint(latitude: -35.363, longitude: 149.165),
                             GeoPoint(latitude: -35.360, longitude: 149.170)]
    expect(MapCentre.mission.enabled(in: planned), "items make the mission entry usable")
    expect(MapCentre.allItems.enabled(in: planned), "and everything covers them too")
    expect(!MapCentre.vehicle.enabled(in: planned),
           "a plan drawn with no vehicle connected still cannot centre on one")

    var fenced = planned
    fenced.otherPoints = [GeoPoint(latitude: -35.40, longitude: 149.20)]
    let missionFrame = MapCentre.mission.frame(in: fenced)
    let allFrame = MapCentre.allItems.frame(in: fenced)
    expect(missionFrame != allFrame,
           "a fence outside the mission gives Everything a different frame from Mission. That is "
           + "all an inequality shows: it cannot say which of the two moved, and nothing here "
           + "pins Mission unchanged")
    expect((allFrame?.latitudeDelta ?? 0) > (missionFrame?.latitudeDelta ?? 0),
           "and Everything is the wider of the two")

    var placed = planned
    placed.launch = GeoPoint(latitude: -35.363, longitude: 149.165)
    placed.vehicle = GeoPoint(latitude: -35.361, longitude: 149.166)
    expect(MapCentre.launch.enabled(in: placed), "a placed launch point can be centred on")
    expect(MapCentre.vehicle.enabled(in: placed), "so can a vehicle that has reported a position")
    expect(MapCentre.launch.frame(in: placed) != MapCentre.vehicle.frame(in: placed),
           "and they are two different places")

    expect(MapCentre.coordinates.frame(in: placed) == nil,
           "typing coordinates is not a frame until the numbers arrive")
    expect(MapCentre.frame(latitude: -35.363, longitude: 149.165) != nil,
           "a real pair gives one")
    expect(MapCentre.frame(latitude: 91, longitude: 0) == nil, "a latitude off the globe does not")
    expect(MapCentre.frame(latitude: 0, longitude: 181) == nil, "nor a longitude")
    expect(MapCentre.frame(latitude: .nan, longitude: 0) == nil, "nor an empty field")

    expect(Set(MapCentre.allCases.map(\.title)).count == MapCentre.allCases.count,
           "no two destinations read alike")
}

func checkFlyOverlays() {
    expect(!FlyOverlays.none.showsOrbit, "a vehicle on the ground draws no orbit")
    expect(!FlyOverlays.none.showsGoto, "nor a fly-to marker")
    expect(FlyOverlays.none.summary, "nothing in progress", "and says so")
    expect(FlyOverlays.none.roiNote, "", "with nothing to explain about the camera")

    let grounded = FlyOverlays.read(orbitCircle: ["center": NSNull()], radius: 0,
                                    orbitActive: false, roiActive: false)
    expect(grounded == FlyOverlays.none,
           "which is exactly what this SITL reports: a null centre, a zero radius and both flags false")

    let flying = FlyOverlays.read(
        orbitCircle: ["center": ["latitude": -35.363 as NSNumber, "longitude": 149.165 as NSNumber]],
        radius: 60, orbitActive: true, roiActive: false)
    expect(flying.showsOrbit, "an active orbit with a centre and a radius is drawn")
    expect(flying.orbitRadius == 60, "at the radius the vehicle reports")

    let noRadius = FlyOverlays.read(
        orbitCircle: ["center": ["latitude": -35.363 as NSNumber, "longitude": 149.165 as NSNumber]],
        radius: 0, orbitActive: true, roiActive: false)
    expect(!noRadius.showsOrbit, "a circle with no radius is not a circle")

    let noCentre = FlyOverlays.read(orbitCircle: ["center": NSNull()], radius: 60,
                                    orbitActive: true, roiActive: false)
    expect(!noCentre.showsOrbit,
           "and an orbit flag with no centre draws nothing rather than a circle at null island")

    let stale = FlyOverlays.read(
        orbitCircle: ["center": ["latitude": -35.363 as NSNumber, "longitude": 149.165 as NSNumber]],
        radius: 60, orbitActive: false, roiActive: false)
    expect(!stale.showsOrbit, "a centre left over from a finished orbit is not drawn either")

    let looking = FlyOverlays.read(orbitCircle: nil, radius: 0, orbitActive: false, roiActive: true,
                                   roi: ["valid": true as NSNumber,
                                         "latitude": -35.363 as NSNumber,
                                         "longitude": 149.165 as NSNumber])
    expect(looking.roiActive, "an ROI the vehicle reports is shown")
    expect(looking.showsRoi, "and marked, now that roiCoord is a property rather than only a signal")
    expect(looking.roiNote.contains("marked spot"), "with the note pointing at the marker")
    expect(looking.summary, "look-at", "which is what is in progress")

    let blind = FlyOverlays.read(orbitCircle: nil, radius: 0, orbitActive: false, roiActive: true,
                                 roi: ["valid": false as NSNumber])
    expect(blind.roiActive, "an ROI with no position is still reported as running")
    expect(!blind.showsRoi, "but nothing is drawn at a place the vehicle did not give")
    expect(blind.roiNote.contains("not given a position"), "and the note says why there is no marker")

    let stopped = FlyOverlays.read(orbitCircle: nil, radius: 0, orbitActive: false, roiActive: false,
                                   roi: ["valid": true as NSNumber,
                                         "latitude": -35.363 as NSNumber,
                                         "longitude": 149.165 as NSNumber])
    expect(!stopped.showsRoi,
           "and a coordinate left over from a finished ROI is not drawn, as with the orbit centre")

    var sent = FlyOverlays.none
    sent.goingTo = GeoPoint(latitude: -35.363, longitude: 149.165)
    expect(sent.showsGoto, "a fly-to the operator issued is marked")
    expect(sent.summary, "fly-to", "and named")

    expect(FlyOverlays.keepsGoto(flightMode: "Guided", gotoFlightMode: "Guided"),
           "the marker survives while the vehicle is still in its goto mode")
    expect(!FlyOverlays.keepsGoto(flightMode: "Loiter", gotoFlightMode: "Guided"),
           "and goes the moment it leaves, which is QGC's onInGotoFlightModeChanged rule")
    expect(!FlyOverlays.keepsGoto(flightMode: "", gotoFlightMode: ""),
           "a vehicle reporting no mode at all keeps nothing")

    var everything = FlyOverlays.read(
        orbitCircle: ["center": ["latitude": -35.363 as NSNumber, "longitude": 149.165 as NSNumber]],
        radius: 60, orbitActive: true, roiActive: true)
    everything.goingTo = GeoPoint(latitude: -35.36, longitude: 149.16)
    expect(everything.summary, "orbit, look-at, fly-to", "all three read out together")
}

func checkMapFollow() {
    let insets = MapInsets(top: 56, left: 24, bottom: 40, right: 352)
    let rect = MapFollow.centreRect(width: 1000, height: 700, insets: insets)
    expect(rect == MapRect(x: 24, y: 56, width: 624, height: 604),
           "the area the panels leave clear is what the vehicle has to stay inside")
    expect(MapFollow.centreRect(width: 300, height: 700, insets: insets) == nil,
           "a window narrower than its own panels leaves no area at all")
    expect(MapFollow.centreRect(width: 0, height: 0, insets: insets) == nil,
           "and neither does one that has not been laid out yet")

    expect(!MapFollow.needsRecentre(vehicle: MapPoint(x: 300, y: 300),
                                    width: 1000, height: 700, insets: insets),
           "a vehicle in the clear area is left where it is, so a pan is not fought")
    expect(MapFollow.needsRecentre(vehicle: MapPoint(x: 800, y: 300),
                                   width: 1000, height: 700, insets: insets),
           "one that has drifted under the side panel is pulled back")
    expect(MapFollow.needsRecentre(vehicle: MapPoint(x: 300, y: 20),
                                   width: 1000, height: 700, insets: insets),
           "so is one above the top inset")
    expect(MapFollow.needsRecentre(vehicle: MapPoint(x: 300, y: 690),
                                   width: 1000, height: 700, insets: insets),
           "and one below the bottom")
    expect(!MapFollow.needsRecentre(vehicle: nil, width: 1000, height: 700, insets: insets),
           "a vehicle that has reported no position moves nothing")
    expect(!MapFollow.needsRecentre(vehicle: MapPoint(x: .nan, y: 300),
                                    width: 1000, height: 700, insets: insets),
           "nor one that projects to nowhere")

    let shift = MapFollow.offset(width: 1000, height: 700, insets: insets)
    expect(shift == MapPoint(x: 164, y: -8),
           "the recentre aims at the middle of the clear area, not the middle of the window")
    expect(MapFollow.offset(width: 0, height: 0, insets: insets) == nil,
           "with no clear area there is nowhere to aim")
    expect(MapFollow.offset(width: 1000, height: 700, insets: .none) == MapPoint(x: 0, y: 0),
           "and with no panels the two middles are the same place")

    expect(MapFollow.follows(setting: true, tracking: true),
           "with the setting on the map follows the vehicle")
    expect(!MapFollow.follows(setting: true, tracking: false),
           "but not before the vehicle has a position")
    expect(!MapFollow.follows(setting: false, tracking: true), "and not with the setting off")
    expect(MapFollow.nudges(setting: false, tracking: true),
           "with it off the map only pulls back when the vehicle leaves the clear area")
    expect(!MapFollow.nudges(setting: true, tracking: true),
           "the two behaviours never both apply")
    expect(!MapFollow.nudges(setting: false, tracking: false), "and neither applies with no vehicle")
}

func checkMyLocation() {
    var state = MapCentreState()
    expect(!MapCentre.myLocation.enabled(in: state),
           "with no ground station position My Location cannot be centred on")
    state.gcs = GeoPoint(latitude: 51.5074, longitude: -0.1278)
    expect(MapCentre.myLocation.enabled(in: state), "with one it can")
    expect(MapCentre.myLocation.frame(in: state) != nil, "and it gives a frame")

    expect(MapCentre.usable(["valid": false as NSNumber,
                             "latitude": 51.5 as NSNumber,
                             "longitude": -0.1 as NSNumber]) == nil,
           "a coordinate the bridge calls invalid is not used even though its numbers look fine")
    expect(MapCentre.usable(["valid": true as NSNumber,
                             "latitude": 0 as NSNumber,
                             "longitude": 0 as NSNumber]) == nil,
           "and null island is rejected on the numbers, because QGeoCoordinate(0,0) calls itself valid")
    expect(MapCentre.usable(["valid": true as NSNumber,
                             "latitude": 51.5074 as NSNumber,
                             "longitude": -0.1278 as NSNumber]) != nil,
           "a real position passes both tests")
    expect(MapCentre.usable(nil) == nil, "and a nested coordinate that came back null is no position")
    expect(MapCentre.usable(["latitude": 51.5 as NSNumber, "longitude": -0.1 as NSNumber]) != nil,
           "a coordinate with no valid key at all is judged on its numbers rather than discarded")
}

func checkViewContract() {
    let root = ProcessInfo.processInfo.environment["QGC_ROOT"] ?? "."
    let path = root + "/test/Bridge/fixtures/view-shapes.json"
    guard let data = FileManager.default.contents(atPath: path),
          let shapes = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else {
        return expect(false, "the recorded view contract is readable at \(path)")
    }

    func base(_ name: String) -> Substring { name.prefix { $0 != "(" } }

    func recordedKey(_ view: String) -> String? {
        guard shapes[view] == nil else { return view }
        let siblings = shapes.keys.filter { base($0) == base(view) }.sorted()
        return siblings.count == 1 ? siblings.first : nil
    }

    func walk(_ view: String, _ inner: [String]) -> Any? {
        inner.reduce(recordedKey(view).flatMap { shapes[$0] }) { here, step in
            guard let dictionary = here as? [String: Any] else { return nil }
            let next = dictionary[step]
            return (next as? [Any])?.first ?? next
        }
    }

    func shape(_ view: String, _ inner: [String]) -> [String: Any]? {
        walk(view, inner) as? [String: Any]
    }

    let recorderSawNoElement =
        "or the recorder saw that list empty and said so. An element shape cannot be pinned from "
        + "a recording that never held an element; QGCCoreCTest's "
        + "_listsRecordedAsEmptyAreCheckedAgainstAVehicle is what checks those against a vehicle"

    func recordedEmpty(_ view: String, _ inner: [String]) -> Bool {
        walk(view, inner) as? String == "empty"
    }

    let required: [(String, [String], [String])] = [
        ("view.plan", ["readiness"], ["ready", "reason"]),
        ("view.plan", ["upload"],
         ["canSend", "canProceed", "pausesFirst", "heading", "proceedTitle", "refusal"]),
        ("view.guidedActions", [], ["connected", "missionActive", "actions"]),
        ("view.guidedActions", ["actions"],
         ["id", "offer", "title", "prompt", "reason", "destructive", "carriesValue"]),
        ("view.guidedAltitude(30)", [],
         ["available", "minimum", "maximum", "current", "label", "unit", "sends", "deltaMeters",
          "sentence"]),
        ("view.guidedTakeoff(10)", [],
         ["available", "minimum", "maximum", "initial", "label", "unit", "targetMeters"]),
        ("view.guidedSpeed(3)", [],
         ["available", "minimum", "maximum", "initial", "label", "unit", "command",
          "targetMetersSecond"]),
        ("view.control(settings.appSettings.audioMuted)", [],
         ["path", "name", "label", "control", "value", "valueString", "display", "units",
          "readOnly", "options", "bits", "decimalPlaces", "minimum", "maximum", "rebootRequired",
          "vehicleRebootRequired", "applicationRestartRequired", "restartNotices"]),
        ("view.track", [],
         ["available", "vehicleId", "recording", "generation", "dropped", "count", "points"]),
        ("view.flyState", [],
         ["connected", "armed", "flying", "landing", "contactLost", "state", "stateText",
          "staleNotice", "mode"]),
        ("view.battery", [], ["level", "packs"]),
        ("view.battery", ["packs"],
         ["level", "text", "secondaryText", "percent", "voltage", "current"]),
        ("view.preflight", [], ["airframe", "groups"]),
        ("view.preflight", ["groups"], ["name", "checks"]),
        ("view.preflight", ["groups", "checks"],
         ["name", "prompt", "verdict", "reason", "blocked"]),
        ("view.warnings", [], ["warnings"]),
        ("view.warnings", ["warnings"], ["id", "text", "detail"]),
        ("view.instruments", [], ["available", "items"]),
        ("view.instruments", ["items"],
         ["id", "group", "name", "label", "value", "units", "missing"]),
        ("view.sensors", [], ["available", "status", "failing", "sensors"]),
        ("view.sensors", ["sensors"], ["name", "state", "label"]),
        ("view.vibration", [],
         ["available", "units", "scaleMaximum", "warningLevel", "dangerLevel", "axes", "worst",
          "clipCounts", "clipping"]),
        ("view.vibration", ["axes"], ["axis", "label", "value", "fraction", "severity"]),
        ("view.settings", ["pages"],
         ["title", "sections", "showsLinks", "showsAbout", "showsVideoSources"]),
        ("view.settings(General)", ["sections"], ["title", "group", "path", "note", "subsections"]),
        ("view.settings(General)", ["sections", "subsections"], ["title", "controls"]),
        ("view.settings(General)", ["sections", "subsections", "controls"],
         ["path", "name", "label", "control", "value", "valueString", "display", "units",
          "readOnly", "rebootRequired", "options", "decimalPlaces", "minimum", "maximum"]),
        ("view.settings(General)", ["sections", "subsections", "controls", "options"],
         ["label", "raw"]),
    ("view.setup", [], ["connected", "firmware", "ready", "headline", "detail", "components",
                        "groups"]),
        ("view.setup", ["groups"], ["title", "pages"]),
        ("view.setup", ["groups", "pages"], ["name", "parameterSections"]),
        ("view.setup(Safety)", [], ["page", "firmware", "available", "sections"]),
        ("view.setup(Safety)", ["sections"], ["title", "note", "controls"]),
        ("view.setup(Safety)", ["sections", "controls"],
         ["path", "name", "label", "control", "valueString", "display", "units", "options"]),
        ("view.video", [],
         ["available", "gstreamer", "streamSource", "decoding", "streaming", "recording",
          "sourceSize", "activeSource", "multipleSources", "anyConnecting", "configuredCount",
          "summary", "cameras"]),
        ("view.video", ["cameras"],
         ["slot", "title", "status", "connecting", "recording", "enabled", "configured"]),
        ("view.camera", [],
         ["present", "title", "model", "vendor", "mode", "modeKnown", "modeText", "isRecording",
          "isTakingPhoto", "stateText", "clockText", "storageStatus", "storageText", "shots",
          "shotsText", "batteryRemaining", "batteryText", "hasZoom", "zoomLevel", "canRecord",
          "canPhoto", "hasModes"]),
        ("view.fences", [], ["polygons", "circles", "rallyPoints", "count"]),
        ("view.fences", ["polygons"],
         ["index", "path", "shape", "inclusion", "kindText", "detailText", "vertices", "usable",
          "framing"]),
        ("view.fences", ["circles"],
         ["index", "path", "shape", "inclusion", "kindText", "detailText", "centre", "centreText",
          "radius", "radiusUnits", "usable", "framing"]),
        ("view.fences", ["rallyPoints"],
         ["index", "path", "latitude", "longitude", "altitude", "altitudeUnits", "altitudePath"]),
        ("view.links", [], ["available", "links", "configured", "linkTypes", "baudRates"]),
        ("view.mapScale(120)", [], ["available", "text", "fraction"]),
        ("view.terrainProfile", [],
         ["usable", "groundKnown", "hasCollision", "unknownTerrain", "minAltitudeMeters",
          "maxAltitudeMeters", "distanceText", "lowestText", "highestText", "points"]),
        ("view.terrainProfile", ["points"],
         ["x", "missionAltitude", "terrainAltitude", "collision"]),
        ("view.missionSummary", [], ["available", "rows", "reason", "altitudeRange"]),
        ("view.missionSummary", ["rows"], ["id", "label", "value"]),
        ("view.modeSlots", [], ["available", "channel", "liveSlot", "slots", "reason"]),
        ("view.modeSlots", ["slots"], ["slot", "mode", "live"]),
        ("view.missionKinds", [], ["kinds"]),
        ("view.missionKinds", ["kinds"],
         ["id", "title", "invokable", "complexName", "geometry", "geometryProperty", "shapeNoun",
          "placementHint", "simple", "enabled", "disabledReason"]),
        ("view.missionSeed(survey,47,8)", [], ["property", "points"]),
        ("view.surveyStats(0)", [],
         ["available", "shotsText", "intervalText", "footprintText", "tooFast", "warning",
          "areaText", "distanceText"]),
        ("view.calibration", [],
         ["connected", "inProgress", "busy", "showsSides", "nextEnabled", "cancelEnabled",
          "progress", "progressText", "helpText", "statusText", "needsAttention", "visibleSides",
          "routines"]),
        ("view.calibration", ["sides"], ["key", "title", "stage", "rotate"]),
        ("view.calibration", ["routines"],
         ["id", "title", "invocation", "arguments", "blocked", "enabled", "description",
          "warning"]),
        ("view.radio", [],
         ["connected", "channelCount", "summary", "shortfall", "calibrating", "statusText",
          "nextText", "nextEnabled", "cancelEnabled", "skipEnabled", "transmitterMode", "channels",
          "sticks"]),
        ("view.radio", ["sticks"],
         ["key", "title", "mapped", "value", "valueText", "fraction", "reversed"]),
        ("view.flightModes", [], ["canSet", "modes"]),
        ("view.flightModes", ["modes"],
         ["name", "summary", "advanced", "current", "needsConfirm"]),
        ("view.inspector", [], ["messages", "rateChoices"]),
        ("view.inspector", ["messages"],
         ["index", "path", "id", "name", "title", "count", "rateText", "targetRateHz",
          "targetRateTitle", "selected"]),
        ("view.inspector", ["rateChoices"], ["rate", "title"]),
        ("view.logs", [],
         ["connected", "requestingList", "downloading", "canRefresh", "canDownload", "canCancel",
          "canErase", "emptyText", "eraseWarning", "entries"]),
        ("view.label(altitudeRelative)", [], ["value"]),
        ("view.messages", [], ["count", "items", "order"]),
        ("view.missionSeed(survey,47,8)", ["points"], ["latitude", "longitude"]),
        ("view.links", ["configured"],
         ["index", "path", "name", "type", "typeLabel", "editing", "displaySummary", "connected",
          "heardVehicle", "statusLine", "autoConnect", "host", "port", "portName", "baud",
          "filename", "logFileName", "lastError"]),
    ]

    let enumerations = (shapes["view.contract"] as? [String: Any])?["enumerations"] as? [String: Any]
    func recorded(_ key: String) -> [String] {
        ((enumerations?[key] as? [Any]) ?? []).compactMap { $0 as? String }
    }

    let actionIds = recorded("view.guidedActions.actions[].id")
    expect(!actionIds.isEmpty, "the core records which guided actions it can name")
    expect(actionIds.filter { GuidedAction(rawValue: $0) == nil }.joined(separator: ","), "",
           "every action the core can emit decodes, because GuidedOffer drops one it cannot name "
           + "and the operator would simply never see that button")
    expect(GuidedAction.allCases.map(\.rawValue).filter { !actionIds.contains($0) }
        .joined(separator: ","), "",
           "and this head carries no action the core never emits")

    expect(recorded("view.guidedActions.actions[].offer").sorted().joined(separator: ","),
           "blocked,hidden,ready",
           "the three offer states this head branches on are the three the core emits; a fourth "
           + "would silently read as shown and unblocked")

    ["view.battery.level", "view.battery.packs[].level"].forEach { key in
        let levels = recorded(key)
        expect(!levels.isEmpty, "the core records \(key)")
        expect(levels.filter { FlyTelemetry.Level($0) == .unknown }.joined(separator: ","), "",
               "every battery level the core emits maps to a colour; an unmapped one drew a grey "
               + "dot for a vehicle-reported low battery once already")
    }

    expect(recorded("view.preflight.groups[].checks[].verdict").sorted().joined(separator: ","),
           "failing,manual,overridable,passing",
           "the four verdicts are the four this head was written against")

    let warningIds = recorded("view.warnings.warnings[].id")
    expect(warningIds.sorted().joined(separator: ","), "noGpsLock,prearm",
           "the two warnings the core can raise are the two the banner was written against")

    expect(InstrumentSelection.defaults.map(\.factName).joined(separator: ","),
           "altitudeRelative,groundSpeed,climbRate,distanceToHome,heading,altitudeAMSL",
           "this head keeps its own copy of the six default readings, because they are needed "
           + "before a bridge read is safe; core-rs instruments::DEFAULTS is the same list in the "
           + "same order, and this pins the copy so the two cannot drift apart unnoticed")

    let sensorStates = recorded("view.sensors.sensors[].state")
    expect(sensorStates.sorted().joined(separator: ","), "disabled,healthy,unhealthy",
           "the three sensor states are the three this head colours; a fourth would take the "
           + "secondary grey of a disabled sensor and read as switched off rather than broken")
    expect(sensorStates.filter { SensorHealth.State($0) == .unknown }.joined(separator: ","), "",
           "and every one of them decodes to a state the row can draw")

    let vibrationSeverities = recorded("view.vibration.axes[].severity")
    expect(vibrationSeverities.sorted().joined(separator: ","), "danger,normal,warning",
           "vibration has its own three severities, which are not the battery's four nor the "
           + "sensors' three, so it is pinned on its own")
    expect(vibrationSeverities.filter { VibrationReading.Severity($0) == nil }.joined(separator: ","), "",
           "and every one of them decodes to a colour the bar can draw")
    expect(recorded("view.vibration.worst").sorted().joined(separator: ","), "danger,normal,warning",
           "the worst level takes the same three, and is absent rather than normal when unknown")

    let controlKinds = recorded("view.control.control")
    expect(controlKinds.sorted().joined(separator: ","), "bitmask,choice,number,text,toggle",
           "the five control kinds are the five this editor draws")
    expect(controlKinds.filter { SettingsControl.Kind($0) == .unknown }.joined(separator: ","), "",
           "and every one of them decodes to an editor, rather than falling through to a field")

    expect(recorded("view.setup.firmware").sorted().joined(separator: ","), "apm,none,px4",
           "the three firmware kinds are the three the setup pages are built for")

    let setupPages = recorded("view.setup.groups[].pages[].name")
    expect(!setupPages.isEmpty,
           "the core records the page names it ships, generated from its own PAGES; this head "
           + "used to keep a hand-copied copy that could not see a page the core added")
    expect(setupPages.filter { SetupPage.symbol(for: $0) == SetupPage.unknownSymbol }
        .sorted().joined(separator: ","), "",
           "every page the core lists has a glyph of its own, so a page added there cannot arrive "
           + "here drawn as a gearshape; Flight Behavior reached the sidebar that way once")
    expect(SetupPage.glyphs.keys.filter { !setupPages.contains($0) }.sorted().joined(separator: ","), "",
           "and this head carries no page the core never lists, which is the direction the old "
           + "test could not check at all because it compared the copy against itself")
    let glyphs = setupPages.map(SetupPage.symbol(for:))
    expect(Set(glyphs).count == glyphs.count,
           "no two pages share an icon, which is how Motors and Remote Support once looked alike")

    let flyStates = recorded("view.flyState.state")
    expect(!flyStates.isEmpty, "the core records the vehicle states it can name")
    expect(flyStates.filter { FlyState.Kind(rawValue: $0) == nil }.sorted().joined(separator: ","),
           "",
           "every state the core can answer decodes to a case here; one it could not name would "
           + "have coloured the status line as an unrecognised state instead")
    expect(FlyState.kinds.map(\.rawValue).filter { !flyStates.contains($0) }
        .sorted().joined(separator: ","), "",
           "and this head carries no state the core never answers, which is the direction a test "
           + "comparing the head's copy against itself could not see")
    expect(!flyStates.contains(FlyState.Kind.unknown.rawValue),
           "unknown is this head's word for a token the core added, never one the core sends")

    let altitudeRaws = ((enumerations?["view.altitudeModes.modes[].raw"] as? [Any]) ?? [])
        .compactMap { ($0 as? NSNumber)?.intValue }
    expect(altitudeRaws.sorted().map(String.init).joined(separator: ","), "0,1,2,3,4",
           "the altitude modes are the numbers of their C++ enum cases; they are written back "
           + "through plan.missionController, so a reordering upstream would silently change "
           + "what every mode means")
    expect(altitudeRaws.filter { !AltitudeMode.raws.contains($0) }.map(String.init)
        .joined(separator: ","), "",
           "every mode the core can offer is one this head names")
    expect(AltitudeMode.raws.filter { !altitudeRaws.contains($0) }.map(String.init)
        .joined(separator: ","), "",
           "and this head names no mode the core never offers, which is the direction that would "
           + "have let a stale constant write a mode that no longer exists")
    expect(recorded("view.altitudeModes.context").sorted().joined(separator: ","), "item,mission",
           "the two menus are the two the core distinguishes")
    expect(recorded("view.altitudeModes.context").filter { !AltitudeMode.contexts.contains($0) }
        .joined(separator: ","), "", "and this head asks for no other")

    let messageLevels = recorded("view.messages.items[].level")
    expect(messageLevels.sorted().joined(separator: ","), "error,normal,warning",
           "the three message levels are the three the vehicle-message row buckets by")
    expect(messageLevels.filter { VehicleMessage.Level(rawValue: $0) == nil }
        .joined(separator: ","), "",
           "and every one decodes; an unrecognised level falls to normal, so a new severity would "
           + "have shown an error in the ordinary colour rather than failing loudly")

    expect(recorded("view.camera.modeText").sorted().joined(separator: ","),
           "Not set,Photo,Survey,Video",
           "the four mode words are the core's, so neither head writes its own")

    expect(recorded("view.fences.polygons[].shape").joined(separator: ","), "polygon",
           "a polygon's shape word is only ever polygon")
    expect(recorded("view.fences.circles[].shape").joined(separator: ","), "circle",
           "and a circle's only ever circle, which is what isCircle reads")

    let stages = recorded("view.calibration.sides[].stage")
    expect(stages.filter { CalibrationSide.Stage($0) == .unknown }.joined(separator: ","), "",
           "every calibration stage the core reports is one this head draws a glyph for, because a "
           + "stage added later would otherwise show a plain circle for a side already done")

    let itemKinds = recorded("view.missionKinds.kinds[].id")
    expect(itemKinds.filter { MissionItemKind.symbol(forId: $0) == MissionKinds.unknownSymbol }
        .joined(separator: ","), "",
           "every item kind the core catalogues has a glyph of its own in this head, because a "
           + "kind added to the core would otherwise appear in the add menu drawn as unknown")
    let geometries = recorded("view.missionKinds.kinds[].geometry")
    expect(geometries.filter { !["area", "line", "null"].contains($0) }.joined(separator: ","), "",
           "and the only geometries are the two the head turns into a property plus none at all")

    let linkKinds = recorded("view.links.links[].type")
    expect(linkKinds.sorted().joined(separator: ","), "bluetooth,logReplay,other,serial,tcp,udp",
           "the six link kinds are the six this head labels")
    let editingModes = recorded("view.links.links[].editing")
    expect(editingModes.sorted().joined(separator: ","),
           "hostAndPort,logFile,none,portOnly,serial",
           "and the five editing modes are the five the form draws")
    expect(editingModes.filter { LinkConfig.Editing($0) == .unknown }.joined(separator: ","), "",
           "every one of them decodes to a form, rather than falling through to editing nothing")

    let neverNull: [(String, [String], [String])] = [
        ("view.battery", ["packs"], ["level", "text", "secondaryText"]),
        ("view.preflight", ["groups", "checks"], ["name", "prompt", "verdict", "reason"]),
        ("view.guidedActions", ["actions"], ["id", "offer", "title", "prompt", "reason"]),
        ("view.warnings", ["warnings"], ["id", "text", "detail"]),
        ("view.instruments", ["items"], ["id", "label", "value", "units"]),
        ("view.sensors", ["sensors"], ["name", "state", "label"]),
        ("view.vibration", ["axes"], ["axis", "label"]),
        ("view.settings(General)", ["sections", "subsections", "controls"],
         ["path", "name", "label", "control", "valueString", "units"]),
        ("view.settings(General)", ["sections", "subsections", "controls", "options"],
         ["label", "raw"]),
        ("view.setup", [], ["firmware", "headline", "detail"]),
        ("view.video", [], ["summary"]),
        ("view.video", ["cameras"], ["title", "status"]),
        ("view.camera", [], ["title", "modeText", "stateText", "storageText", "shotsText"]),
        ("view.links", ["configured"], ["name", "typeLabel", "displaySummary", "statusLine"]),
        ("view.mapScale(120)", [], ["text"]),
        ("view.terrainProfile", [],
         ["distanceText", "lowestText", "highestText", "minAltitudeMeters", "maxAltitudeMeters"]),
        ("view.missionKinds", ["kinds"], ["title", "shapeNoun", "placementHint"]),
        ("view.surveyStats(0)", [],
         ["shotsText", "intervalText", "footprintText", "warning", "areaText", "distanceText"]),
        ("view.calibration", [], ["progressText", "helpText", "statusText", "needsAttention"]),
        ("view.calibration", ["routines"], ["title", "description", "invocation"]),
        ("view.radio", [], ["summary", "shortfall", "statusText", "nextText"]),
        ("view.radio", ["sticks"], ["title", "valueText"]),
        ("view.flightModes", ["modes"], ["name", "summary"]),
        ("view.inspector", ["messages"], ["title", "rateText", "targetRateTitle"]),
        ("view.inspector", ["rateChoices"], ["title"]),
        ("view.logs", [], ["emptyText", "eraseWarning"]),
        ("view.label(altitudeRelative)", [], ["value"]),
    ]
    neverNull.forEach { view, inner, keys in
        let place = inner.isEmpty ? view : "\(view).\(inner.joined(separator: "."))"
        guard let shown = shape(view, inner) else {
            return expect(recordedEmpty(view, inner),
                          "\(place) is in the recorded contract, " + recorderSawNoElement)
        }
        let nullable = keys.filter { key in
            let recorded = (shown[key] as? String) ?? ""
            return recorded == "null" || recorded.hasPrefix("null|")
        }.sorted()
        expect(nullable.joined(separator: ","), "",
               "this head renders \(place) straight into the interface with no fallback, so a "
               + "field the core can send as null would put an empty row in front of the operator")
    }

    required.forEach { view, inner, keys in
        let where_ = inner.isEmpty ? view : "\(view).\(inner.joined(separator: "."))"
        guard let recorded = shape(view, inner) else {
            return expect(recordedEmpty(view, inner),
                          "\(where_) is in the recorded contract, " + recorderSawNoElement)
        }
        let missing = keys.filter { recorded[$0] == nil }.sorted()
        expect(missing.joined(separator: ","), "",
               "every key this head decodes from \(where_) is one the core actually emits")
    }
}

func checkGuidedOffers() {
    func offer(_ id: String, _ state: String, _ reason: String = "") -> [String: Any] {
        ["id": id, "title": "T", "prompt": "P is the prompt", "offer": state, "reason": reason,
         "destructive": (id == "emergencyStop") as NSNumber, "carriesValue": (id == "takeoff") as NSNumber]
    }

    let list = GuidedOffer.list([offer("arm", "blocked", "The vehicle's arming checks are failing."),
                                 offer("takeoff", "ready"),
                                 offer("land", "hidden"),
                                 offer("emergencyStop", "ready")])
    expect(list.count == 4, "every action the core describes is carried across")
    expect(list.filter(\.shown).map(\.id).joined(separator: ","), "arm,takeoff,emergencyStop",
           "a hidden action is not shown, and a blocked one still is, because it has something to say")
    expect(list.filter(\.ready).map(\.id).joined(separator: ","), "takeoff,emergencyStop",
           "but only an unblocked action can be commanded")

    let arm = list[0]
    expect(arm.blocked, "a blocked action is disabled")
    expect(!list[1].blocked,
           "and one the core says is ready is not, or nothing could ever be commanded at all")
    expect(arm.explanation, "The vehicle's arming checks are failing.",
           "and explains itself with the core's reason rather than the generic prompt")
    expect(list[1].explanation, "P is the prompt",
           "while a ready one says what it will do")
    expect(list[3].destructive, "the emergency stop stays marked destructive")
    expect(list[1].carriesValue, "and takeoff still asks for a number")

    expect(GuidedOffer.list(nil).isEmpty, "no actions is not a crash")
    expect(GuidedOffer(["id": "notAnAction", "offer": "ready"]) == nil,
           "an id this head does not know is dropped, not guessed at")
    expect(GuidedOffer(["id": "arm"]) == nil,
           "and an entry with no offer state is dropped rather than read as shown")

    let invented = GuidedOffer(offer("arm", "awaitingConfirmation"))
    expect(invented?.ready == false,
           "an offer state this head has never heard of is NOT ready -- readiness is derived from "
           + "the core saying ready, not from it failing to say blocked, so a fourth state added "
           + "upstream cannot hand an operator a live Arm button")
    expect(invented?.blocked == true,
           "it reads as blocked, so the button is disabled rather than enabled")
    expect(invented?.shown == true,
           "but it is still drawn, because an action that exists and cannot be used is worth "
           + "showing greyed with a tooltip rather than hiding entirely")

    expect(GuidedOffer(["id": "takeoff", "offer": "ready", "carriesValue": true as NSNumber])?
        .carriesValue == true,
           "the field is carriesValue, which checkViewContract now pins against the core's own "
           + "recorded shape rather than against a spelling agreed only with itself")
}

func checkMapScale() {
    func bar(_ overrides: [String: Any]) -> MapScaleBar? {
        MapScaleBar(["available": true as NSNumber, "text": "100 m",
                     "fraction": 1.0 as NSNumber].merging(overrides) { _, new in new })
    }

    expect(bar([:])?.text ?? "", "100 m", "the bar reads the label the core snapped")
    expect(bar(["fraction": 1.25 as NSNumber])?.fraction == 1.25,
           "and the share of the measured width the core says to draw")

    expect(bar(["available": false as NSNumber]) == nil,
           "a width the core could not snap draws no bar at all")
    expect(MapScaleBar([:]) == nil, "and neither does a read that returned nothing")
    expect(bar(["fraction": NSNull()]) == nil,
           "fraction arrives null whenever available is false, so it is never read as a zero-length "
           + "bar drawn over the map")

    expect(MapScaleBar.across(0) == nil, "a map that has not laid out yet asks for no scale")
    expect(MapScaleBar.across(.nan) == nil,
           "nor one measuring nothing \u{2014} and the guard runs before the width becomes an Int, "
           + "because Int(Double.nan) is a trap and not a zero")
    expect(MapScaleBar.across(.infinity) == nil, "nor one measuring everything")
    expect(MapScaleBar.across(120.0) == 120, "a real width is asked for as a whole number of metres")
}

func checkTerrainDownload() {
    expect(!TerrainDownload.none.started, "a vehicle that has fetched no terrain shows no panel")
    expect(TerrainDownload.none.text, "", "and says nothing")

    let busy = TerrainDownload(loaded: 120, pending: 380)
    expect(busy.busy, "blocks still pending means a download is running")
    expect(busy.total == 500, "the total is what has landed plus what is still coming")
    expect(busy.fraction == 0.24, "and the bar fills by that ratio, as QGC's pctComplete does")
    expect(busy.percentText, "24%", "shown as a whole percent")
    expect(busy.text, "Loading terrain 120 of 500", "with the counts spelled out")

    let done = TerrainDownload(loaded: 504, pending: 0)
    expect(!done.busy, "nothing pending means nothing is downloading")
    expect(done.started, "but 504 blocks is still something to report")
    expect(done.fraction == 1, "a finished download fills the bar")
    expect(done.text, "Terrain loaded, 504 blocks", "and says how many arrived")
    expect(TerrainDownload(loaded: 1, pending: 0).text, "Terrain loaded, 1 block",
           "with the singular spelled properly")

    expect(TerrainDownload.showing(busy, sinceIdle: nil),
           "a running download is shown whether or not it has ever been idle")
    expect(TerrainDownload.showing(busy, sinceIdle: 999),
           "and a stale idle stamp does not hide one that started again")
    expect(TerrainDownload.showing(done, sinceIdle: 5),
           "a finished download lingers, which is QGC's thirty second timer")
    expect(TerrainDownload.showing(done, sinceIdle: 29.9), "right up to the last moment")
    expect(!TerrainDownload.showing(done, sinceIdle: 30), "and goes at thirty seconds")
    expect(!TerrainDownload.showing(done, sinceIdle: nil),
           "a finished download with no idle stamp is not shown, so a restart does not resurrect it")
    expect(!TerrainDownload.showing(.none, sinceIdle: 1), "and nothing at all is never shown")

    let read = TerrainDownload.read([["name": "blocksLoaded", "value": 504 as NSNumber],
                                     ["name": "blocksPending", "value": 0 as NSNumber]])
    expect(read == done, "which is exactly what this grounded vehicle reports")
    expect(TerrainDownload.read([]) == .none, "and a vehicle with no terrain facts reports nothing")
    expect(TerrainDownload.read([["name": "blocksLoaded"]]) == .none,
           "as does one whose fact carries no value")
}

func checkPolygonEdit() {
    func corner(_ lat: Double, _ lon: Double) -> [String: Any] {
        ["latitude": lat as NSNumber, "longitude": lon as NSNumber]
    }
    let polygon = EditablePolygon([
        "path": "p", "ring": true as NSNumber, "closed": true as NSNumber,
        "minimumVertices": 3 as NSNumber, "canRemoveVertex": true as NSNumber,
        "segments": 4 as NSNumber, "splitInvokable": "splitPolygonSegment",
        "adjustInvokable": "adjustVertex", "removeInvokable": "removeVertex",
        "vertices": [corner(0, 0), corner(0, 2), corner(2, 2), corner(2, 0)],
        "midpoints": [corner(0, 1), corner(1, 2), corner(2, 1), corner(1, 0)],
    ])
    expect(polygon?.ring == true,
           "the core says whether the shape closes back on itself, which decides polygon or "
           + "polyline on the map. Its sibling `closed` was decoded here and consumed by NOTHING, "
           + "so it is gone: it does NOT mean the path loops -- fences.rs computes it as "
           + "`vertices.len() >= minimum`, so a two-point corridor line reports closed true with "
           + "ring false. A head reading it as the geometric term would loop a corridor back on "
           + "itself")
    expect(polygon?.canRemoveVertex == true, "and whether one can go without breaking it")
    expect(polygon?.segments == 4, "a ring has as many segments as corners")
    expect(polygon?.midpoints.count == 4,
           "with a midpoint on each, which the core places rather than this head averaging pairs")
    expect(polygon?.splitInvokable ?? "", "splitPolygonSegment",
           "a ring splits with the polygon call, and the core names it")

    let line = EditablePolygon([
        "path": "l", "ring": false as NSNumber, "closed": true as NSNumber,
        "minimumVertices": 2 as NSNumber, "canRemoveVertex": false as NSNumber,
        "segments": 1 as NSNumber, "splitInvokable": "splitSegment",
        "vertices": [corner(0, 0), corner(0, 2)], "midpoints": [corner(0, 1)],
    ])
    expect(line?.segments == 1, "a two-point line has one segment, not two")
    expect(line?.canRemoveVertex == false,
           "and neither end can go, because two points are the minimum the core states")

    expect(PolygonEdit.splits(0, in: polygon!), "a segment index inside the ring can be split")
    expect(!PolygonEdit.splits(4, in: polygon!), "one past the last cannot")
    expect(PolygonEdit.removes(0, in: polygon!), "a corner can be removed from a square")
    expect(!PolygonEdit.removes(0, in: line!), "but not from a two-point line")

    expect(PolygonEdit.removalHint(polygon!), "Remove this corner",
           "a corner that can go says so; the head had no way to remove one at all, so the core "
           + "answered canRemoveVertex and nothing asked")
    expect(PolygonEdit.removalHint(line!), "A line needs at least 2 corners",
           "and one that cannot says why, in the core's own minimum rather than QGC's literal 3")

    let triangle = EditablePolygon([
        "path": "t", "ring": true as NSNumber, "closed": true as NSNumber,
        "minimumVertices": 3 as NSNumber, "canRemoveVertex": false as NSNumber,
        "segments": 3 as NSNumber,
        "vertices": [corner(0, 0), corner(0, 2), corner(2, 0)],
    ])
    expect(PolygonEdit.removalHint(triangle!), "A shape needs at least 3 corners",
           "a shape at its minimum names a shape, not a line")

    expect(EditablePolygon(["ring": true as NSNumber]) == nil,
           "a polygon with no path is dropped, because the path is what an edit is invoked on")
    expect(EditablePolygon(nil) == nil, "and no answer is no polygon")
}

func checkPlanViewState() {
    let json: [String: Any] = [
        "readiness": ["ready": false, "reason": "Waiting for terrain heights before the plan can be saved or sent."],
        "upload": ["canSend": false, "canProceed": true, "pausesFirst": true,
                   "heading": "Upload this plan?", "proceedTitle": "Pause and upload",
                   "refusal": "This vehicle is flying this mission."],
    ]
    let readiness = PlanReadiness(json["readiness"])
    expect(readiness?.ready == false, "the head takes the core's answer, it does not recompute it")
    expect(readiness?.reason.contains("terrain") == true, "including the sentence the operator reads")

    let upload = PlanUpload(json["upload"])
    expect(upload?.canSend == false, "a refusal does not send")
    expect(upload?.canProceed == true, "but this one the operator may accept")
    expect(upload?.pausesFirst == true, "after a pause, so the vehicle stops flying items being replaced")
    expect(upload?.proceedTitle ?? "", "Pause and upload", "and the button says the pause out loud")

    expect(PlanReadiness(nil) == nil, "a missing object is not silently a ready plan")
    expect(PlanUpload("not an object") == nil, "nor is a shape the core never sends")

    expect(PlanReadiness(["reason": "unset"])?.ready == false,
           "a readiness object that carries no answer is NOT ready. It gates Save As and Upload, "
           + "and defaulting it to ready enabled both before anything had been read")

    expect(PlanUpload(["refusal": "unset"])?.canSend == false,
           "and an upload object that carries no answer cannot send")
    expect(PlanUpload(["refusal": "unset"])?.canProceed == false,
           "nor can the operator wave it through")

    let noVehicle = PlanUpload(["canSend": false as NSNumber,
                                "refusal": "No vehicle is connected, so there is nowhere to send this plan."])
    expect(noVehicle?.refusal ?? "", "No vehicle is connected, so there is nowhere to send this plan.",
           "the button says why it will not send, and the reason comes from the question it asked")
    expect(noVehicle?.canSend == false, "and the same answer decides whether it is enabled at all")

    expect(PlanUpload.unknown.canSend == false,
           "an upload state the core never answered refuses, because a plan is not sendable until "
           + "something said it was -- this is the value the store holds before its first read")
    expect(PlanUpload.unknown.refusal, PlanUpload.uncheckable,
           "and it says the check itself did not happen rather than inventing a refusal that "
           + "sounds like the vehicle's")

    expect(PlanUpload.uncheckable.contains("not sent"),
           "when the upload check cannot be read at all the plan is not sent and the refusal "
           + "says so; uploadToVehicle used to treat an unreadable check as permission, so a "
           + "bridge that could not answer sent the plan rather than refusing it")
}

func checkCentreNotes() {
    var state = MapCentreState()
    expect(MapCentre.mission.note(in: state) == "This plan has no waypoints",
           "a greyed centre option says why rather than leaving the operator guessing")
    expect(MapCentre.vehicle.note(in: state) == "No vehicle position yet",
           "and stays true whether a vehicle is connected or not, rather than implying one is")
    expect(MapCentre.coordinates.note(in: state) == "",
           "the option that is always available needs no note")

    state.access = .notAsked
    expect(MapCentre.myLocation.note(in: state).contains("not been asked"),
           "an unanswered permission prompt is something the operator can act on")
    state.access = .refused
    expect(MapCentre.myLocation.note(in: state).contains("not allowing"),
           "a refused permission reads differently from one never asked for")
    state.access = .waiting
    expect(MapCentre.myLocation.note(in: state) == "Waiting for a position fix",
           "with permission granted the wait is the honest reason")
    state.access = .unknown
    expect(MapCentre.myLocation.note(in: state) == "No position for this computer",
           "and an unknown status claims nothing about permission")

    state.gcs = GeoPoint(latitude: 1, longitude: 2)
    expect(MapCentre.myLocation.enabled(in: state), "a usable position enables the option")
    expect(MapCentre.myLocation.note(in: state) == "",
           "an enabled option carries no note at all")
}

func checkMenuPlacement() {
    let width = 220.0, screen = 1000.0
    expect(MapMenuPlacement.place(click: 100, extent: width, container: screen) == 118,
           "the menu opens just past the cursor, not pinned to a corner")
    expect(MapMenuPlacement.place(click: 900, extent: width, container: screen) == 662,
           "near the far edge it flips to the other side of the cursor")
    expect(MapMenuPlacement.place(click: 5, extent: width, container: screen) == 23,
           "a click at the near edge still leaves the menu inside the margin")
    expect(MapMenuPlacement.place(click: 400, extent: 2000, container: screen) == 12,
           "a menu larger than the window sits at the margin rather than off-screen")
    let far = MapMenuPlacement.place(click: 995, extent: width, container: screen)
    expect(far + width <= screen - MapMenuPlacement.margin + 0.001,
           "a flipped menu never runs past the far edge")
}

func checkMessageRate() {
    func choice(_ rate: Int, _ title: String) -> [String: Any] {
        ["rate": rate as NSNumber, "title": title]
    }
    let choices = MessageRateChoice.list([
        choice(-1, "Off"), choice(0, "Default"), choice(1, "1 Hz"), choice(2, "2 Hz"),
        choice(3, "3 Hz"), choice(4, "4 Hz"), choice(5, "5 Hz"), choice(6, "6 Hz"),
        choice(7, "7 Hz"), choice(8, "8 Hz"), choice(9, "9 Hz"), choice(10, "10 Hz"),
        choice(25, "25 Hz"), choice(50, "50 Hz"), choice(100, "100 Hz"),
    ])

    expect(choices.count == 15, "QGC offers fifteen rates and the core lists all of them")
    expect(choices.first?.title ?? "", "Off",
           "asking for no messages reads as Off rather than minus one, and is first as QGC orders it")
    expect(choices[1].title, "Default", "then the vehicle's own choice, not a rate of zero")
    expect(Set(choices.map(\.rate)).count == choices.count, "and none is listed twice")

    expect(MessageRateChoice.offered(25, in: choices), "25 Hz is one of them")
    expect(!MessageRateChoice.offered(17, in: choices),
           "17 Hz is not, so the picker will not send it")
    expect(!MessageRateChoice.offered(-5, in: choices), "nor is a negative that is not Off")
    expect(!MessageRateChoice.offered(0, in: []),
           "and before the choices have loaded nothing is offered, so no rate can be sent")

    expect(MessageRateChoice.shown(4, in: choices) == 4,
           "a rate the vehicle reports and the picker offers is shown as itself")
    expect(MessageRateChoice.shown(17, in: choices) == MessageRateChoice.defaultRate,
           "and a rate the picker cannot show falls back to Default rather than selecting nothing")
    expect(MessageRateChoice.shown(MessageRateChoice.offRate, in: choices)
        == MessageRateChoice.offRate, "Off shows as Off")

    expect(MessageRateChoice.offered(MessageRateChoice.defaultRate, in: choices),
           "the rate that shown() falls back to is itself one the picker lists; if the core ever "
           + "dropped Default from its choices the control would bind to a tag that is not there "
           + "and draw blank, which is the failure this pins rather than the fallback value")
    expect(choices.map(\.rate).map(String.init).joined(separator: ","),
           "-1,0,1,2,3,4,5,6,7,8,9,10,25,50,100",
           "and the fifteen are core-rs inspector.rs RATE_CHOICES in its order, so this fixture "
           + "cannot quietly stop being what the core actually sends")

    expect(MessageRateChoice(["title": "5 Hz"]) == nil, "a choice with no rate is dropped")
}

func checkHostNotices() {
    func notice(_ id: Int, _ kind: Any, _ title: String, _ text: String,
                _ at: Any = NSNull()) -> [String: Any] {
        ["id": id as NSNumber, "kind": kind, "title": title, "text": text, "at": at]
    }

    let read = HostNotices(["dropped": 0 as NSNumber, "notices": [
        notice(1, "vehicleError", "AircastQGC", "Critical: GPS glitch"),
        notice(2, "message", "AircastQGC", "Mission transfer failed. Error: timeout"),
        notice(3, "navigation", "setup", ""),
    ]])
    expect(read.all.count == 3, "every notice the core queued is carried across")
    expect(read.shown.map(\.id).map(String.init).joined(separator: ","), "2",
           "but only the app's own message is drawn. A vehicleError comes from "
           + "StatusTextHandler::newErrorMessage, which fires on the same STATUSTEXT that "
           + "reaches vehicle.formattedMessages, so the Fly view's message panel already has it; "
           + "drawing it again would show one fault twice")
    expect(read.shown.first?.line ?? "",
           "AircastQGC: Mission transfer failed. Error: timeout",
           "and the line carries the core's title beside its text")

    expect(HostNotice(notice(9, "invented", "T", "body"))?.kind == .unknown,
           "a kind token this head has never heard of is unknown, not the first case")
    expect(HostNotice(notice(9, "invented", "T", "body"))?.shows == true,
           "and it is DRAWN: the core meant to say something, and this head not recognising the "
           + "word is no reason to lose the sentence. Silence is the failure this channel exists "
           + "to fix")
    expect(HostNotice(notice(9, NSNull(), "T", "body"))?.kind == .unknown,
           "so is a notice with no kind at all")

    expect(HostNotice(["kind": "message", "title": "T", "text": "b"]) == nil,
           "a notice with no id is dropped, because the id is what acknowledge sends back and "
           + "one that cannot be acknowledged would sit there forever")

    expect(HostNotice(notice(4, "message", "", "just text"))?.line ?? "", "just text",
           "a message with no title reads as its text alone rather than a leading colon")
    expect(HostNotice(notice(5, "navigation", "setup", ""))?.line ?? "", "setup",
           "and one with no text reads as its title")

    let stamped = HostNotice(notice(6, "message", "T", "b", 1_757_000_000_000 as NSNumber))
    expect(stamped?.at != nil, "the core sends milliseconds since the epoch and the head reads them")
    expect(HostNotice(notice(7, "message", "T", "b"))?.at == nil,
           "and a notice with no stamp has no time rather than one at the epoch")

    expect(HostNotices(["dropped": 3 as NSNumber, "notices": []]).dropNotice ?? "",
           "3 earlier notices were dropped.",
           "the queue caps at 64 and drops from behind its oldest eight, so what goes missing is "
           + "the middle of a burst; a count the operator never sees would let the list disagree "
           + "with what happened")
    expect(HostNotices(["dropped": 1 as NSNumber, "notices": []]).dropNotice ?? "",
           "1 earlier notice was dropped.", "with the singular spelled properly")
    expect(HostNotices.none.dropNotice == nil, "and nothing dropped says nothing")
    expect(HostNotices([:]).all.isEmpty && HostNotices([:]).dropped == 0,
           "an empty read is an empty queue rather than a decode that invents one")

    expect(read.newest?.id == 2,
           "the newest drawn notice is the last one queued, because post appends and the head "
           + "does not re-sort what the core has already ordered")

    expect(read.offersSetup,
           "the navigation notice offers Vehicle Setup rather than being discarded. QGC posts it "
           + "from AutoPilotPlugin.cc:72 when a component needs setup, alongside the sentence "
           + "that explains why -- and that sentence is already drawn, so this is a destination "
           + "and not another row saying the same thing")
    expect(read.unreachable == nil, "and a destination this head can reach needs no apology")

    let elsewhere = HostNotices(["notices": [notice(1, "navigation", "joystickConfig", "")]])
    expect(elsewhere.offersSetup == false, "a destination this head has never heard of is not setup")
    expect(elsewhere.unreachable ?? "",
           "The app asked to open joystickConfig, which this window cannot reach.",
           "and it SAYS SO, naming the word, because a request that was made and cannot be "
           + "honoured is not a request that can be dropped in silence")
    expect(elsewhere.shown.isEmpty,
           "while still drawing no row for it: the title is a destination token, not a sentence, "
           + "and \"joystickConfig\" in front of an operator is worse than the line above")

    expect(HostNotices.none.offersSetup == false && HostNotices.none.unreachable == nil,
           "an empty queue asks for nothing and apologises for nothing")

    let served = HostNotices(["notices": [notice(1, "navigation", "setup", ""),
                                          notice(2, "message", "AircastQGC", "needs setup")]])
    let unreadable = HostNotices(["count": 2 as NSNumber, "notices": [NSNull(), NSNull()]])
    expect(unreadable.all.isEmpty, "a list of nulls decodes to no notices")
    expect(unreadable.lostNotice ?? "", "2 notices could not be read.",
           "and the head SAYS SO, because the producer counted two. Measured on the running app: "
           + "host.count answered 2 while host.notices answered [null, null] -- a QVariantList "
           + "property is a leaf the bridge cannot walk. compactMap swallowed both and drew an "
           + "empty panel, which is the silence this channel exists to end")
    expect(HostNotices(["count": 1 as NSNumber, "notices": [NSNull()]]).lostNotice ?? "",
           "1 notice could not be read.", "with the singular spelled properly")
    expect(read.lostNotice == nil,
           "while a queue whose notices all decode has lost nothing, so the line stays away")
    expect(HostNotices(["notices": [notice(1, "message", "T", "b")]]).lostNotice == nil,
           "and a producer that sends no count accuses nobody of losing anything -- the clamp "
           + "does that, not a fallback. I first wrote the missing count as the list's own "
           + "length, and the break proved that unreachable: max(0, ...) already made the two "
           + "spellings identical, so the assertion could not have failed either way")
    expect(HostNotices(["count": 1 as NSNumber,
                        "notices": [notice(1, "message", "T", "b"),
                                    notice(2, "message", "T", "b")]]).lost == 0,
           "a count SMALLER than the list clamps to nothing lost rather than to a negative. The "
           + "probe reports this number, and a diagnostic reading minus one would send whoever "
           + "read it looking for a bug in the wrong half")
    expect(HostNotices(["count": 1 as NSNumber,
                        "notices": [notice(1, "vehicleError", "T", "b")]]).lostNotice == nil,
           "a notice this head chooses not to draw is not one it failed to read; lost counts "
           + "against what DECODED, not against what is shown")

    expect(served.navigation?.id == 1,
           "the navigation request is found among notices that are not navigation, because it "
           + "arrives paired with the message that explains it and never alone")
}

func checkVehicleMessageOrder() {
    func message(_ text: String, _ level: String = "normal") -> [String: Any] {
        ["text": text, "time": "12:00:0\(text)", "level": level]
    }
    let seven = (1...7).map { message("\($0)") }

    let oldestFirst = VehicleMessages(["order": "oldestFirst", "items": seven])
    expect(oldestFirst.newest(6).map(\.text).joined(separator: ","), "7,6,5,4,3,2",
           "the core serves its lists oldest first and says so. This head took the FRONT and "
           + "called it the latest, so past the limit the operator never saw a new message again "
           + "-- the panel froze on the first six the vehicle ever sent")

    let newestFirst = VehicleMessages(["order": "newestFirst", "items": seven])
    expect(newestFirst.newest(6).map(\.text).joined(separator: ","), "1,2,3,4,5,6",
           "and if the core ever turns a list around it says which way, so this head reads the "
           + "answer rather than carrying an assumption that was right once")

    expect(VehicleMessages(["items": seven]).order == .oldestFirst,
           "a list that does not say takes the core's stated contract, which is what every one of "
           + "its lists answers today")
    expect(VehicleMessages(["order": "sideways", "items": seven]).order == .oldestFirst,
           "and so does one that says something this head does not know, rather than falling "
           + "through to the end that happens to be listed first here")

    expect(VehicleMessage.worst(oldestFirst.newest(6)) == .normal, "six ordinary messages are ordinary")
    let shouting = VehicleMessages(["order": "oldestFirst",
                                    "items": seven + [message("8", "error")]])
    expect(VehicleMessage.worst(shouting.newest(6)) == .error,
           "an error that has just arrived is the worst of the window. The probe's worstMessage "
           + "reads this same window -- nothing in the interface does, each row draws its own "
           + "colour -- so taking the wrong end had it reporting on the six oldest messages and "
           + "an error the vehicle had just raised could not reach it")

    expect(VehicleMessages(["order": "oldestFirst", "items": [message("1")]])
        .newest(6).map(\.text).joined(separator: ","), "1",
           "a list shorter than the limit is taken whole rather than padded or trimmed to nothing")
    expect(VehicleMessages.empty.newest(6).isEmpty, "and an empty one stays empty")
}

func checkFlownLeg() {
    let roi = MissionItem(view: ["index": 2, "sequence": 2, "name": "Region of interest",
                                 "flownLeg": false,
                                 "coordinate": ["latitude": -35.33, "longitude": 149.25]], selected: -1)
    let waypoint = MissionItem(view: ["index": 1, "sequence": 1, "name": "Waypoint",
                                      "flownLeg": true,
                                      "coordinate": ["latitude": -35.35, "longitude": 149.20]], selected: -1)
    expect(roi.hasPosition && waypoint.hasPosition,
           "both earn a marker, because both are somewhere on the map")
    expect([roi, waypoint].filter(\.flownLeg).map(\.sequence).map(String.init).joined(separator: ","),
           "1",
           "but only one is a leg the vehicle flies. Measured on the running app: a plan of "
           + "takeoff, waypoint, ROI, waypoint reported four placed items and drew one line "
           + "through all four, so the route doglegged out to the ROI and back")

    let unplaced = MissionItem(view: ["index": 3, "sequence": 3, "name": "Delay"], selected: -1)
    expect(!unplaced.flownLeg && !unplaced.hasPosition,
           "an item with no place at all is neither a marker nor a leg")

    func leg(_ seq: Int, _ name: String, flown: Bool, ends: Bool = false) -> MissionItem {
        MissionItem(view: ["index": seq, "sequence": seq, "name": name,
                           "flownLeg": flown, "endsRoute": ends,
                           "coordinate": ["latitude": -35.3 - Double(seq) / 100,
                                          "longitude": 149.2]], selected: -1)
    }
    let past = [leg(0, "Mission Start", flown: true), leg(2, "Waypoint", flown: true),
                MissionItem(view: ["index": 3, "sequence": 3, "name": "Return To Launch",
                                   "flownLeg": false, "endsRoute": true], selected: -1),
                leg(4, "Waypoint", flown: true)]
    expect(MissionItem.route(past).map(\.sequence).map(String.init).joined(separator: ","), "0,2",
           "the route stops where the mission does. The waypoint after the return is in the plan "
           + "and the vehicle never gets there, so a leg drawn to it is a line across the map")
    expect(past.filter(\.hasPosition).count == 3,
           "and all three placed items keep their markers, because the plan really does contain "
           + "them and an operator editing one has to see it")

    let finishing = [leg(0, "Mission Start", flown: true), leg(1, "Waypoint", flown: true),
                     leg(2, "Land", flown: true, ends: true)]
    expect(MissionItem.route(finishing).map(\.sequence).map(String.init).joined(separator: ","),
           "0,1",
           "a landing ends the route at the leg before it; nothing is drawn onward from where the "
           + "mission finishes")

    let plain = [leg(0, "Mission Start", flown: true), leg(1, "Waypoint", flown: true)]
    expect(MissionItem.route(plain).count == 2,
           "and a plan that never ends explicitly keeps every leg it has")

    let atOrigin = MissionItem(view: ["index": 4, "sequence": 4, "name": "Waypoint",
                                      "flownLeg": true,
                                      "coordinate": ["latitude": 0, "longitude": 0]], selected: -1)
    expect(atOrigin.hasPosition,
           "and a coordinate the core did send is taken at face value, even at 0,0. The head used "
           + "to throw those away itself because an unplaced command reported one; the core sends "
           + "no coordinate for those now, so re-adding the guard here would only lose a waypoint "
           + "somebody really put in the Gulf of Guinea")
}

func checkBatteryReading() {
    let live = BatteryReading(["voltage": 15.812 as NSNumber, "voltageText": "15.81V",
                               "currentText": "3.41A", "percentText": "76%"])
    expect(live?.voltageText ?? "", "15.81V",
           "every measure on this panel is the string the CORE spelled, from the vehicle's own "
           + "Fact. This head used to format all three with String(format:), which is "
           + "locale-independent and prints a full stop where the Fact prints whatever the "
           + "operator's locale does -- and the Fly view was already drawing the core's "
           + "spelling, so one battery was written two ways in one app")
    expect(live?.currentText ?? "", "3.41A", "the current likewise")
    expect(live?.percentText ?? "", "76%", "and the charge")
    expect(live?.available == true, "a pack reporting a voltage is a pack there is a reading for")

    expect(BatteryReading.unavailable.available == false,
           "no reading at all is not a reading of zero, which would draw a flat pack")
    expect(BatteryReading.unavailable.voltageText, BatteryReading.unreported, "it shows a dash")

    let partial = BatteryReading(["voltage": 15.0 as NSNumber, "voltageText": "15.00V"])
    expect(partial?.currentText ?? "", BatteryReading.unreported,
           "a measure the core did not spell reads as absent rather than as an empty row -- and "
           + "the head no longer has a formatter to fall back to, which is the point: it cannot "
           + "invent a spelling that disagrees with the one beside it")
    expect(partial?.available == true, "a pack with a voltage and no current still has a voltage")

    expect(BatteryReading(["voltage": Double.nan as NSNumber])?.available == false,
           "a nan voltage is not a voltage, so the panel says there is no reading rather than "
           + "drawing one whose number cannot be compared")
    expect(BatteryReading(nil) == nil, "no pack at all is no reading")
}

func checkShapeAbsence() {
    expect(PlanShapeAbsence.fence(connected: false, supported: false),
           PlanShapeAbsence.noFence,
           "with no vehicle the plan simply has no geofence. Saying the firmware lacks the feature "
           + "would be a claim about a vehicle nothing has looked at")
    expect(PlanShapeAbsence.fence(connected: false, supported: true), PlanShapeAbsence.noFence,
           "and it reads the same either way, because what a disconnected vehicle supports is not "
           + "something this window knows")
    expect(PlanShapeAbsence.fence(connected: true, supported: true),
           "No geofence. Nothing will stop the vehicle leaving the area.",
           "a connected vehicle that could hold one has not been given one, and the consequence "
           + "is the point of saying so")
    expect(PlanShapeAbsence.fence(connected: true, supported: false),
           "This vehicle's firmware does not support geofences.",
           "and one that cannot hold one says so instead -- telling an operator to draw a fence "
           + "their firmware will not take is worse than saying nothing, because they will try")

    expect(PlanShapeAbsence.rally(connected: false, supported: false), PlanShapeAbsence.noRally,
           "the rally tab answers the same three ways")
    expect(PlanShapeAbsence.rally(connected: true, supported: true),
           "No rally points. On a failsafe the vehicle returns to launch.",
           "and names what happens without one, which is the reason to add one")
    expect(PlanShapeAbsence.rally(connected: true, supported: false),
           "This vehicle's firmware does not support rally points.",
           "and does not invite an operator to add what cannot be taken")
}

func checkSensorsComponentIsFoundByClass() {
    func component(name: String, className: String) -> VehicleComponentInfo? {
        VehicleComponentInfo(["name": name, "className": className,
                              "needsAttention": false as NSNumber])
    }
    expect(component(name: "センサ", className: "SensorsComponent")?.isSensors ?? false,
           "the sensors component is recognised by its class, because matching its name meant "
           + "the fault badge never appeared in the five locales that translate Sensors")
    expect(component(name: "센서", className: "APMSensorsComponent")?.isSensors ?? false,
           "and ArduPilot's own sensors component is the same component to this window")
    expect(!(component(name: "Radio", className: "RadioComponent")?.isSensors ?? true),
           "another component is not it, whatever it is called")
    expect(component(name: "センサ", className: "SensorsComponent")?.id ?? "", "SensorsComponent",
           "identity comes from the class too -- keying it on a translated name would have "
           + "rebuilt every row the moment the language changed")
}

func checkComplexGeometryInAnyLocale() {
    let catalogue = MissionKinds(["kinds": [
        ["id": "survey", "title": "Survey", "complexName": "Survey", "geometry": "area",
         "geometryProperty": "surveyAreaPolygon", "simple": false as NSNumber, "enabled": true as NSNumber],
        ["id": "corridor", "title": "Corridor Scan", "complexName": "Corridor Scan", "geometry": "line",
         "geometryProperty": "corridorPolyline", "simple": false as NSNumber, "enabled": true as NSNumber],
    ]])
    func item(_ kind: String, named: String) -> MissionItem {
        MissionItem(view: ["index": 1 as NSNumber, "sequence": 1 as NSNumber,
                           "name": named, "kind": kind], selected: -1)
    }
    expect(catalogue.areaProperty(of: item("survey", named: "調査")) ?? "", "surveyAreaPolygon",
           "a survey draws its area from the item's kind, not from a name that is translated -- "
           + "matching the name found nothing outside an English build and the polygon that "
           + "tells the operator what ground is covered was simply absent")
    expect(catalogue.lineProperty(of: item("corridor", named: "回廊スキャン")) ?? "", "corridorPolyline",
           "and a corridor draws its path the same way")
    expect(catalogue.areaProperty(of: item("corridor", named: "Corridor Scan")) == nil,
           "a line is never offered as an area, whatever it is called")
    expect(catalogue.lineProperty(of: item("survey", named: "調査")) == nil,
           "an area is not offered as a line, which would append vertices to the wrong property")
    expect(catalogue.areaProperty(of: item("waypoint", named: "Waypoint")) == nil,
           "and a kind the catalogue does not shape has no geometry to read")
}

func checkSpeedChangeIsListedNotOnlySelected() {
    func item(speed: String?, blocked: String? = nil) -> MissionItem {
        var view: [String: Any] = ["index": 2 as NSNumber, "sequence": 2 as NSNumber,
                                   "name": "Waypoint", "kind": "waypoint"]
        view["speedChangeText"] = speed
        view["blockedReason"] = blocked
        return MissionItem(view: view, selected: -1)
    }
    expect(item(speed: "12.0 m/s").subtitle(unreached: false), "Flies at 12.0 m/s",
           "an item that commands its own speed says so in the list. The core builds the "
           + "measure -- specifiedFlightSpeed through the operator's speed setting, so 23.3 kn "
           + "where that is the setting -- and this head only wraps it in a sentence. Before "
           + "this the figure existed on the selected item alone, so reviewing a thirty-item "
           + "plan for speed changes meant selecting all thirty")
    expect(item(speed: nil).subtitle(unreached: false), "",
           "and an item that commands no speed says nothing, rather than a sentence about the "
           + "speed it inherits, which would put a line under every row in the plan")
    expect(item(speed: "").subtitle(unreached: false), "",
           "an empty string is not a speed either: the core withholds a commanded zero because "
           + "\"0.0 m/s\" reads as an instruction to stop, and \"Flies at \" reads as a defect")
    expect(item(speed: "12.0 m/s", blocked: "Set its location").subtitle(unreached: false),
           "Set its location",
           "a block outranks it. The subtitle is one line and the reasons are ordered by what "
           + "the operator must do about them: a block is a task, and burying it under a speed "
           + "would lose the only thing stopping the plan being saved")
    expect(item(speed: "12.0 m/s").subtitle(unreached: true), "Never flown to",
           "and so does never being flown to, which says the item is not on the route at all -- "
           + "the speed it would have commanded there is not the point")
}

func checkAnAltitudeSaysWhatItIsMeasuredFrom() {
    func item(_ frame: Any, text: String = "541 m", kind: String = "survey",
              word: String? = nil) -> MissionItem {
        var view: [String: Any] = ["index": 3 as NSNumber, "sequence": 3 as NSNumber,
                                   "name": kind, "kind": kind, "altitudeBandText": text,
                                   "altitudeFrame": frame]
        if let word { view["altitudeFrameText"] = word }
        return MissionItem(view: view, selected: -1)
    }
    expect(item("amsl", word: "AMSL").altitudeReading, "541 m AMSL",
           "the column was showing 491 m, 75.0 m and 541 m together with nothing saying which "
           + "was measured from where -- a launch height above the sea, a waypoint above the "
           + "launch pad, and a survey band above the sea, with 75 four pixels from 541. The "
           + "core now names the frame and the two that are not the operator's default say so")
    expect(item(MissionItem.launchFrame, text: "75.0 m", kind: "waypoint", word: "")
        .altitudeReading,
           "75.0 m",
           "and the common case stays bare: a number with no frame reads as relative to launch, "
           + "which is what QGC has always meant by it. Labelling all three would put a word "
           + "under every row to disambiguate the two that need it")
    expect(item("terrain", word: "AGL").altitudeReading, "541 m AGL",
           "a terrain-following item is the THIRD frame, not a second one -- altitude_frame "
           + "returns amsl, launch or terrain, and a head treating this as a boolean would leave "
           + "the terrain case wearing the label of whichever branch it fell into")
    expect(item("gundeck").altitudeReading, "541 m GUNDECK",
           "and a frame NEITHER side has heard of is SHOWN, not swallowed -- even when the core "
           + "declines to name it. 7f624ad2d made frame_word return Option, so an unnamed token "
           + "now serialises as JSON NULL and this fixture omits the key to match -- it used to "
           + "send \"\", which was what the core did before and is what launch-relative still "
           + "sends. Those two must not collapse: empty means NO SUFFIX, absent means NO IDEA. "
           + "The head keeps its uppercase fallback for exactly the second, and Android has the "
           + "identical test under the name SEABED")
    let terrainWaypoint = MissionItem(
        view: ["index": 2 as NSNumber, "sequence": 2 as NSNumber, "name": "Waypoint",
               "kind": "waypoint", "specifiesAltitude": true as NSNumber,
               "altitudeText": "75.0 m", "altitudeUnits": "m", "altitudeFrame": "terrain",
               "altitudeFrameText": "AGL"],
        selected: -1)
    expect(terrainWaypoint.altitudeFieldUnits, "m AGL",
           "an EDITABLE item carries a frame too, and this is the case that nearly got away. A "
           + "waypoint set to Calculated Above Terrain draws through AltitudeField, not through "
           + "altitudeReading, so the suffix never reached it: measured on the running app, a "
           + "terrain waypoint and a launch-relative takeoff both read \"75.0 m\" and are "
           + "different heights over rising ground. The frame rides the units beside the editor")
    expect(MissionItem(view: ["index": 1 as NSNumber, "sequence": 1 as NSNumber,
                              "name": "Takeoff", "kind": "takeoff",
                              "specifiesAltitude": true as NSNumber, "altitudeUnits": "m",
                              "altitudeFrame": MissionItem.launchFrame,
                              "altitudeFrameText": ""], selected: -1)
        .altitudeFieldUnits, "m",
           "and the default frame adds nothing to the unit, so the common editable row is "
           + "unchanged -- the whole point of labelling only the exceptions")
    expect(terrainWaypoint.mapSubtitle ?? "", "75.0 m AGL",
           "the MAP MARKER spells the altitude too, and it was the third place -- I added the "
           + "frame to the list row and to the editable field and stopped, because those were "
           + "the two I had been looking at. The Android head hit the same shape on a summary "
           + "chip: adding a qualifier to a value means finding EVERY place that value is "
           + "spelled, and the count is never one, because you have already fixed the one you "
           + "were thinking of")
    expect(item("amsl", text: "").mapSubtitle == nil,
           "and an item with no altitude gets NO subtitle rather than a pin labelled with an em "
           + "dash: the annotation had been comparing against the dash by hand, in one of two "
           + "spellings of the same character used a few lines apart")
    expect(item("amsl", text: "").altitudeReading, MissionItem.noAltitude,
           "an item with no altitude at all keeps its em dash and gains no frame: there is no "
           + "measurement to say the reference of")
}

func checkSaveAsksTheCoreRatherThanTheReadiness() {
    let busy = PlanActions(["open": true as NSNumber, "save": true as NSNumber,
                            "exportKml": true as NSNumber, "newPlan": true as NSNumber,
                            "clearMission": false as NSNumber, "addFence": true as NSNumber,
                            "addRally": true as NSNumber])
    expect(busy?.save == true,
           "the Save button was disabled whenever readiness said not ready, and readiness is "
           + "about an item still being drawn. Measured: an eight-item plan with a half-drawn "
           + "takeoff had readyToSave false and the menu greyed out -- and saving it through the "
           + "probe wrote 92943 bytes. The head was blocking an action that works and telling "
           + "the operator their plan could not be saved")
    expect(PlanActions(["save": NSNull()])?.save == false,
           "an answer the core did not give is not a yes: a missing or null flag disables rather "
           + "than enables, so a view that failed to answer cannot open a door")
    expect(PlanActions("not an object") == nil,
           "and a payload that is not an object yields no actions at all, rather than seven "
           + "falses that look like a considered refusal")
    expect(busy?.clearFromVehicle == false,
           "the core's key is clearMission and this head names it clearFromVehicle, because that "
           + "is what it is: its test says clearing needs a vehicle to clear it FROM. The Clear "
           + "button in this window calls plan.removeAll and empties the local plan, which needs "
           + "no vehicle -- wiring the two together by name would disable a working action "
           + "whenever nothing is connected, which here is always")
    expect(busy?.exportKml == true,
           "Export KML was gated on items.count < 2, a count this head made up. It happens to "
           + "agree with the core in both states measured, which is how a duplicated derivation "
           + "survives every sweep: nothing is wrong on screen until the producer changes its "
           + "mind and only one of the two follows")
}

func checkAnUnreadStoreDoesNotBlameTheVehicle() {
    expect(FenceSupport.unread.refusal(servedReason: "") ?? "", FenceSupport.unreadRefusal,
           "a store that has never read says so. fenceSupported was a Bool defaulting to false, "
           + "and addFence guarded on it, so before the Plan window appeared the head answered "
           + "\"This vehicle does not accept a geofence\" -- its OWN sentence, about a vehicle "
           + "that had refused nothing. I measured that, believed it, told both peers fences "
           + "were unreachable here, and wrote the refusal into a rig as expected")
    expect(FenceSupport.answered(false).refusal(servedReason: "") ?? "",
           FenceSupport.unsupportedRefusal,
           "and a store that HAS read and been told no says the LINK does not accept one. This "
           + "assertion used to say \"the vehicle ... because then it is true\", and that "
           + "reasoning was wrong: GeoFenceController::supported is a capability bit AND "
           + "maxProtoVersion >= 200, so a false can mean the vehicle lacks the feature or that "
           + "the link speaks MAVLink 1. Naming the vehicle picks one cause without reading "
           + "either. The core spells all four cases and its sentence is preferred whenever it "
           + "has one; this is only the fallback")
    expect(FenceSupport.answered(true).refusal(servedReason: "") == nil,
           "a supported fence has nothing to refuse")
    expect(!FenceSupport.unread.offers,
           "unread offers nothing either -- the button is not drawn as available on the strength "
           + "of a value nobody has fetched")
    expect(FenceSupport.answered(true).offers,
           "and an answered yes does, which is the state the running app is actually in: measured "
           + "after the Plan window appears, fenceSupported is true and addFence succeeds with no "
           + "vehicle attached")
    expect(FenceSupport.unread != FenceSupport.answered(false),
           "unread and answered-no are different states carrying different sentences, which is "
           + "the whole point: collapsing them to one Bool is what let a default speak for a "
           + "vehicle")
}

func checkAPatternDrawsThePathItActuallyFlies() {
    func at(_ latitude: Double, _ longitude: Double) -> [String: Any] {
        ["latitude": latitude as NSNumber, "longitude": longitude as NSNumber]
    }
    let view: [String: Any] = ["items": [
        ["sequence": 2 as NSNumber, "kind": "waypoint", "geometry": NSNull()],
        ["sequence": 3 as NSNumber, "kind": "survey",
         "geometry": ["shape": "area", "property": "surveyAreaPolygon",
                      "vertices": [at(47.0, 8.0), at(47.1, 8.0), at(47.1, 8.1)],
                      "transects": [at(47.01, 8.01), at(47.09, 8.01), at(47.09, 8.02)]]],
        ["sequence": 4 as NSNumber, "kind": "structure",
         "geometry": ["shape": "area", "property": "structurePolygon",
                      "vertices": [at(47.2, 8.2), at(47.3, 8.2), at(47.3, 8.3)],
                      "transects": []]],
    ]]
    let found = PatternGeometry.all(view)
    expect(found.count == 2,
           "a waypoint carries no geometry at all and the core sends null for it, so two of the "
           + "three items answer -- the head does not invent an empty shape for an item that has "
           + "none")
    expect(PatternGeometry.flownLines(found).count == 1,
           "and only the survey contributes a flown line. Measured on a real plan: a survey "
           + "carries 112 transect points and a corridor 8, while a structure scan carries NONE "
           + "-- it circles a shape rather than mowing it. The third member of the class again, "
           + "and a head assuming every pattern has transects would draw a line through nothing")
    expect(PatternGeometry.flownLines(found).first?.count == 3,
           "the points come through in the order the core walked them, which is the order the "
           + "vehicle flies them -- a set or a re-sort would draw the serpentine inside out")
    expect(PatternGeometry(["shape": "", "transects": [at(47.0, 8.0), at(47.1, 8.1)]]) == nil,
           "a geometry with no shape is refused rather than drawn as an unnamed line: shape is "
           + "what says whether the vertices close into an area or run as a path")
    expect(PatternGeometry.flownLines([PatternGeometry(
        ["shape": "area", "transects": [at(47.0, 8.0)]])!]).isEmpty,
           "and a single point is not a line. One transect point cannot be flown between, and "
           + "MKPolyline with one coordinate draws nothing while still costing an overlay")

    let scan = PatternGeometry(["shape": "area", "property": "structurePolygon",
                                "vertices": [at(47.0, 8.0), at(47.0, 8.1), at(47.1, 8.1)],
                                "transects": [],
                                "flightLoop": [at(47.0, 8.0), at(47.0, 8.1), at(47.1, 8.1)]])!
    expect(PatternGeometry.flownLines([scan]).first?.count == 4,
           "a structure scan draws the loop it FLIES, and this head drew no route at all for one. "
           + "Measured on the running app: the core serves transects as an EMPTY list for a "
           + "structure scan -- it has no visualTransectPoints -- and flownLines filtered the "
           + "empty list out, so the map drew the outline the operator dragged round the building "
           + "and nothing through it. Three corners become FOUR points because a loop is CLOSED: "
           + "the core serves the corners only, first != last, and drawing them open leaves one "
           + "side of the building with no route over it, which reads as a side the aircraft "
           + "does not fly")
    expect(PatternGeometry.flownLines([scan]).first?.last == PatternGeometry.flownLines([scan]).first?.first,
           "and the closing point is the starting one, not a copy of the last corner")

    let survey = PatternGeometry(["shape": "area", "transects": [at(47.0, 8.0), at(47.1, 8.1)],
                                  "flightLoop": [at(1.0, 1.0), at(2.0, 2.0), at(3.0, 3.0)]])!
    expect(PatternGeometry.flownLines([survey]).first?.count == 2,
           "and transects still win where an item has both. A survey is flown as open sweeps, "
           + "not a ring, so closing it would draw a leg from the last transect back to the "
           + "first that the aircraft never flies")

    expect(PatternGeometry.areas(found).count == 2,
           "BOTH the survey and the structure scan are areas, though only one of them is flown "
           + "as transects. The outline a structure scan encloses is the thing the operator drew "
           + "and has to see; gating the outline on having transects would erase it from the map "
           + "and leave a survey-shaped hole where a building is")
    expect(PatternGeometry.lines(found).isEmpty,
           "and neither of them is a line -- the shape says which, not the presence of transects")

    let corridor = PatternGeometry(["shape": "line", "property": "corridorPolyline",
                                    "vertices": [at(47.0, 8.0), at(47.2, 8.2)],
                                    "transects": []])!
    expect(PatternGeometry.lines([corridor]).count == 1 && PatternGeometry.areas([corridor]).isEmpty,
           "a corridor's two vertices are a path and never an area: three points is the floor "
           + "for something that encloses ground, two for something that merely runs across it")
    expect(PatternGeometry.areas([PatternGeometry(
        ["shape": "area", "vertices": [at(47.0, 8.0), at(47.1, 8.1)]])!]).isEmpty
           && PatternGeometry.lines([PatternGeometry(
        ["shape": "line", "vertices": [at(47.0, 8.0)]])!]).isEmpty,
           "and each threshold refuses the degenerate case one short of it -- two corners "
           + "enclose no ground and one point crosses none, both of which MapKit renders as "
           + "nothing while still costing an overlay")
}

func checkAPatternSaysHowHighAboveTheGroundItFlies() {
    expect(SurveyStats(["available": true as NSNumber, "shotsText": "1043",
                        "surfaceDistanceText": "50.0 m"]).surfaceDistanceText, "50.0 m",
           "a survey flies a fixed height above the GROUND, and until the core named it the "
           + "operator had no way to read it at all: a TransectStyleComplexItem has no altitude "
           + "fact, so the row's altitude column is an em dash and correctly so")
    expect(SurveyStats(["available": true as NSNumber,
                        "surfaceDistanceText": NSNull()]).surfaceDistanceText, "",
           "an explicit null leaves it empty and the row is not drawn, rather than a labelled "
           + "blank claiming the survey flies at no height above anything")
    expect(SurveyStats(["available": true as NSNumber]).surfaceDistanceText, "",
           "and so does an absent key, which is the other of the three states the bridge carries")
    let survey = MissionItem(view: ["index": 4 as NSNumber, "sequence": 4 as NSNumber,
                                    "name": "Survey", "kind": "survey",
                                    "altitudeText": NSNull(), "altitudeBandText": NSNull()],
                             selected: -1)
    expect(survey.altitudeReading, MissionItem.noAltitude,
           "and the height above the ground must NOT land in the item's altitude column. Those "
           + "are different quantities -- one measured from the terrain, one from the launch "
           + "point, and for a terrain-following survey they differ BY the terrain. The Android "
           + "head drew cameraCalc.distanceToSurface there and the row read 50.0 m as though it "
           + "were an altitude; 97f05e940 is the core naming the quantity so neither head has to "
           + "guess. An em dash is the honest answer for a column this item has no value for")
}

func checkAnItemSaysHowManyCommandsItFolds() {
    func item(folds: Int?, speed: String? = nil) -> MissionItem {
        var view: [String: Any] = ["index": 2 as NSNumber, "sequence": 2 as NSNumber,
                                   "name": "Survey", "kind": "survey"]
        view["foldedCommands"] = folds.map { $0 as NSNumber }
        view["speedChangeText"] = speed
        return MissionItem(view: view, selected: -1)
    }
    expect(item(folds: 141).subtitle(unreached: false), "141 more commands",
           "a survey at sequence 2 is followed by a row numbered 144, and nothing on screen "
           + "accounted for the 141 in between. Measured: a survey folds 141, a corridor 11. "
           + "The operator was left to read a hole in the numbering as a defect")
    expect(item(folds: 0).subtitle(unreached: false), "",
           "an item that folds nothing says nothing -- every plain waypoint reports zero, and a "
           + "row saying so under each of them is noise that hides the two rows where it matters")
    expect(item(folds: nil).subtitle(unreached: false), "",
           "and an item that never reported a last sequence is silent rather than claiming it "
           + "folds none: the core sends null there instead of zero, precisely so the two stay "
           + "distinguishable, and a head collapsing them throws that away")
    expect(MissionItem(view: ["index": 2 as NSNumber, "sequence": 2 as NSNumber,
                              "name": "Survey", "kind": "survey",
                              "foldedCommands": NSNull(), "extraSeconds": NSNull(),
                              "speedChangeText": NSNull()], selected: -1)
        .subtitle(unreached: false), "",
           "and an EXPLICIT null is silent too, which is the case that actually travels: the "
           + "core distinguishes value, null and absent, and a key held at null is what it "
           + "really sends. Assigning nil to a Swift dictionary REMOVES the key, so every other "
           + "assertion here tests the absent branch and none of them crossed the boundary the "
           + "fallback exists for")
    expect(item(folds: 1, speed: "9.0 m/s").subtitle(unreached: false),
           "Flies at 9.0 m/s \u{00B7} 1 more command",
           "the singular loses its s. A waypoint that commands a speed folds exactly one -- the "
           + "DO_CHANGE_SPEED itself -- so this is the case that explains the commonest gap, and "
           + "it reads alongside the speed rather than instead of it")
}

func checkARouteLeavesAPatternWhereItEnds() {
    func place(_ sequence: Int, _ kind: String, _ latitude: Double, _ longitude: Double,
               exit: (Double, Double)? = nil) -> MissionItem {
        var view: [String: Any] = [
            "index": sequence as NSNumber, "sequence": sequence as NSNumber,
            "name": kind, "kind": kind, "flownLeg": true as NSNumber,
            "coordinate": ["latitude": latitude as NSNumber, "longitude": longitude as NSNumber]]
        if let exit {
            view["exitCoordinate"] = ["latitude": exit.0 as NSNumber,
                                      "longitude": exit.1 as NSNumber]
        }
        return MissionItem(view: view, selected: -1)
    }
    func drawn(_ items: [MissionItem]) -> String {
        MissionItem.routePoints(items)
            .map { String(format: "%.4f,%.4f", $0.latitude, $0.longitude) }
            .joined(separator: " -> ")
    }
    let survey = place(2, "survey", 47.4024, 8.5482, exit: (47.3998, 8.5521))
    expect(drawn([place(1, "waypoint", 47.3995, 8.5480), survey,
                  place(3, "waypoint", 47.4030, 8.5560)]),
           "47.3995,8.5480 -> 47.4024,8.5482 -> 47.3998,8.5521 -> 47.4030,8.5560",
           "a survey is entered at one corner and left at another -- measured 411 m apart on a "
           + "real one -- so the leg to the next item starts where the pattern ENDS. Drawing it "
           + "from the entry doubles the route back across the ground the survey just covered, "
           + "which is the core's own words for why exitCoordinate exists")
    expect(drawn([place(1, "waypoint", 47.3995, 8.5480), place(2, "waypoint", 47.4024, 8.5482)]),
           "47.3995,8.5480 -> 47.4024,8.5482",
           "a waypoint is left where it was entered, so it contributes ONE point. The core sends "
           + "no exitCoordinate at all in that case -- it filters an exit equal to the entry "
           + "rather than making every head dedupe a point drawn on top of a point")
    expect(MissionItem.routePoints([survey]).count == 2,
           "the pattern alone is still two points: entering and leaving are both real positions "
           + "even with nothing after it to fly to")
    expect(drawn([survey, place(1, "waypoint", 47.3995, 8.5480)])
            != drawn([place(1, "waypoint", 47.3995, 8.5480), survey]),
           "and the two points are ordered entry then exit rather than collected as a set -- "
           + "a reversed pair draws the leg backwards through the pattern")
}

func checkAnItemSaysEverythingItDoes() {
    func item(speed: String? = nil, hold: Double? = nil, blocked: String? = nil) -> MissionItem {
        var view: [String: Any] = ["index": 2 as NSNumber, "sequence": 2 as NSNumber,
                                   "name": "Waypoint", "kind": "waypoint"]
        view["speedChangeText"] = speed
        view["extraSeconds"] = hold.map { $0 as NSNumber }
        view["blockedReason"] = blocked
        return MissionItem(view: view, selected: -1)
    }
    expect(item(hold: 15).subtitle(unreached: false), "Holds for 15 s",
           "a waypoint that waits says how long. The core serves additionalTimeDelay as a raw "
           + "number and not as text, deliberately -- seconds have no unit preference, so unlike "
           + "a speed there is nothing for it to convert and the spelling is the head's")
    expect(item(hold: 0).subtitle(unreached: false), "",
           "every item carries a delay of zero, so a row saying \"Holds for 0 s\" would appear "
           + "under every waypoint in the plan and mean nothing. Withholding it here is the same "
           + "judgement the core makes about a commanded speed of zero, made on the other side")
    expect(item(hold: 2.5).subtitle(unreached: false), "Holds for 2.5 s",
           "a fraction keeps its fraction, and a whole number loses the .0 a Double would print")
    expect(item(hold: Double(Float(2.7))).subtitle(unreached: false), "Holds for 2.7 s",
           "a hold that has been through a vehicle keeps a bounded number of digits. A mission "
           + "item's param1 is a float32 on the wire, so a plan DOWNLOADED from a vehicle widens "
           + "2.7 to 2.700000047683716, and String(Double) prints every one of those digits -- the "
           + "row read \"Holds for 2.700000047683716 s\". The local editing path never showed it "
           + "because the core serves a clean 2.7 when the value came from a fact this head set, "
           + "measured on the running app. Swift's Double description is a debugging spelling, "
           + "never a drawn measurement")
    expect(item(hold: 1e30).subtitle(unreached: false), "Holds for 1000000000000000019884624838656 s",
           "and an absurd number is spelled, not crashed on. String(Int(seconds)) would trap here "
           + "-- the value is past Int.max -- so a payload nobody expects would take down the plan "
           + "list rather than draw a silly row. A formatter cannot trap")
    expect(item(speed: "12.0 m/s", hold: 15).subtitle(unreached: false),
           "Flies at 12.0 m/s \u{00B7} Holds for 15 s",
           "an item that does BOTH says both. These are not competing claims to one line the way "
           + "a block and a speed are -- they are two true facts about the same item, and ranking "
           + "them would drop one. Read in the order they happen: the speed governs the flying, "
           + "the hold is what it does on arrival")
    expect(item(speed: "12.0 m/s", hold: 15, blocked: "Set its location").subtitle(unreached: false),
           "Set its location",
           "but a block still outranks both together, because it is the only one of the three "
           + "that is a task rather than a description")
}

func checkOnlyAPlacedItemMoves() {
    func item(_ sequence: Int, _ kind: String, movable: Bool, placed: Bool) -> MissionItem {
        var view: [String: Any] = ["index": sequence as NSNumber, "sequence": sequence as NSNumber,
                                   "name": kind, "kind": kind, "movable": movable as NSNumber]
        if placed {
            view["coordinate"] = ["latitude": -35.0 as NSNumber, "longitude": 149.0 as NSNumber]
        }
        return MissionItem(view: view, selected: -1)
    }
    expect(item(2, "waypoint", movable: true, placed: true).canMove,
           "a placed waypoint is drawn, so it can be dragged")
    expect(!item(1, "takeoff", movable: false, placed: false).canMove,
           "an unplaced takeoff is drawn nowhere and must not be moved: writing its coordinate "
           + "leaves it unplaced and relocates the launch point instead, because "
           + "TakeoffMissionItem::setCoordinate also writes the settings item for a multirotor")
    expect(!item(0, "settings", movable: false, placed: true).canMove,
           "the plan's own entry is not dragged even though it reports a coordinate")
    expect(item(3, "waypoint", movable: true, placed: false).canMove,
           "and the head takes the core's answer rather than second-guessing it from a position "
           + "the view may not have sent")
}

func checkUnreachedItems() {
    func item(_ index: Int, _ kind: String, ends: Bool) -> MissionItem {
        MissionItem(view: ["index": index as NSNumber, "sequence": index as NSNumber,
                           "name": kind, "kind": kind,
                           "endsRoute": ends as NSNumber, "flownLeg": true as NSNumber,
                           "coordinate": ["latitude": -35.0 as NSNumber,
                                          "longitude": 149.0 as NSNumber]], selected: -1)
    }
    let plain = [item(0, "waypoint", ends: false), item(1, "waypoint", ends: false)]
    expect(MissionItem.unreached(plain).isEmpty,
           "a plan that never ends its route leaves every item reachable")

    let endsLast = plain + [item(2, "command", ends: true)]
    expect(MissionItem.unreached(endsLast).isEmpty,
           "a return to launch that is last leaves nothing after it, which is why every plan "
           + "shape built through the editor agreed before this case was tried")

    let past = endsLast + [item(3, "waypoint", ends: false), item(4, "roi", ends: false)]
    expect(MissionItem.unreached(past) == [3, 4],
           "the vehicle turns for home at the return to launch, so both items after it are "
           + "uploaded and neither is reached -- the map already drew no leg to them while the "
           + "list presented them as ordinary waypoints")
    expect(!MissionItem.route(endsLast).isEmpty,
           "the fixture carries real positions, without which route() filters everything out and "
           + "the comparison below compares nothing to nothing")
    expect(MissionItem.route(past).count == MissionItem.route(endsLast).count,
           "and the legs drawn are unchanged by appending past the end, because both answers "
           + "come from one routeEnd rather than two copies of the same rule")
}

func checkUnknownMissionTime() {
    func summary(_ rows: [[String: Any]]) -> MissionSummary {
        MissionSummary(["available": true as NSNumber, "rows": rows, "reason": ""])
    }

    let known = summary([["id": "distance", "label": "Distance", "value": "26.10 km"],
                         ["id": "time", "label": "Time", "value": "1:27:24"]])
    expect(known.timeText, "1:27:24",
           "the controller's own answer is drawn unchanged when it has one")

    let vtol = summary([["id": "distance", "label": "Distance", "value": "26.10 km"]])
    expect(vtol.describes,
           "the fixture still has rows, without which the strip draws nothing at all and the "
           + "assertion below would pass for the wrong reason")
    expect(vtol.timeText, MissionSummary.unknown,
           "a VTOL's speed changes at the transition, so the core declines rather than guessing "
           + "-- and a chip that vanished instead would leave the operator unable to tell that "
           + "from having missed it")
}

func checkGroundDrawnFromKnownSamples() {
    func sample(_ x: Double, terrain: Double?) -> [String: Any] {
        var json: [String: Any] = ["x": x as NSNumber, "missionAltitude": 100.0 as NSNumber,
                                   "sequence": 0 as NSNumber]
        if let terrain { json["terrainAltitude"] = terrain as NSNumber }
        return json
    }
    func profile(_ points: [[String: Any]]) -> TerrainProfile {
        TerrainProfile(["points": points, "usable": true as NSNumber])
    }

    let whole = profile([sample(0, terrain: 50), sample(0.5, terrain: 60), sample(1, terrain: 70)])
    expect(whole.groundRuns.count == 1, "ground sampled throughout is one filled run")
    expect(whole.groundRuns.first?.count == 3,
           "carrying every sample, without which the assertions below compare empty to empty")

    let oneMissing = profile([sample(0, terrain: 50), sample(0.1, terrain: nil),
                              sample(0.5, terrain: 60), sample(1, terrain: 70)])
    expect(oneMissing.groundRuns.count == 1,
           "one unplaced item leaves its sample without terrain, and that must not erase the "
           + "ground everywhere else -- the core's groundKnown means every sample is known, so "
           + "gating the whole drawing on it threw away four hundred measured samples for one")
    expect(oneMissing.groundRuns.first?.count == 2,
           "the run resumes after the gap rather than swallowing it")

    let gap = profile([sample(0, terrain: 50), sample(0.2, terrain: 55), sample(0.4, terrain: nil),
                       sample(0.6, terrain: 60), sample(0.8, terrain: 65)])
    expect(gap.groundRuns.count == 2,
           "and a gap in the middle breaks the fill in two, because one shape spanning it would "
           + "draw ground at a height nothing measured")
}

func checkAltitudeModePickerIsOffered() {
    let offers = AltitudeMode.offers(["modes": [["raw": 1 as NSNumber, "title": "Relative To Launch",
                                                 "enabled": true as NSNumber, "reason": ""],
                                                ["raw": 2 as NSNumber, "title": "AMSL",
                                                 "enabled": true as NSNumber, "reason": ""]]])
    expect(offers.count == 2,
           "the fixture carries the modes the assertions below depend on, without which every one "
           + "of them passes because the list is empty rather than because the rule works")

    expect(AltitudeMode.offersPicker(mode: 1, in: offers),
           "an item that measures its altitude somehow gets the picker")
    expect(!AltitudeMode.offersPicker(mode: AltitudeMode.none, in: offers),
           "a mission start entry has no altitude of its own, so the core offers modes when asked "
           + "and the item still has none -- gating on the offers alone drew a picker with "
           + "nothing selected and no way to tell it from a broken control")
    expect(!AltitudeMode.offersPicker(mode: 1, in: []),
           "and nothing is drawn when the core offers no modes at all")
}

func checkAltitudeReading() {
    func item(_ kind: String, specifies: Bool, text: String, band: String = "") -> MissionItem {
        MissionItem(view: ["index": 0 as NSNumber, "sequence": 0 as NSNumber,
                           "name": kind, "kind": kind,
                           "specifiesAltitude": specifies as NSNumber,
                           "altitudeText": text, "altitudeBandText": band], selected: -1)
    }

    expect(item("waypoint", specifies: true, text: "75.0 m").altitudeReading, "75.0 m",
           "an item that measures its altitude reads the core's own sentence")
    expect(item("settings", specifies: false, text: "585 m").altitudeReading, "585 m",
           "and the mission start entry keeps its reading: it specifies no altitude of its own, "
           + "but the figure it carries is the planned home altitude, which is a real height")
    expect(item("command", specifies: false, text: "0.0 m").altitudeReading,
           MissionItem.noAltitude,
           "a DO_ command has no altitude, so the zero in its unused parameter slot is not one -- "
           + "it was drawn as 0.0 m, a plausible height for an item that has none")
    expect(item("survey", specifies: false, text: "", band: "585 m to 660 m").altitudeReading,
           "585 m to 660 m",
           "a survey flies a band rather than one height, so specifiesAltitude is absent on it "
           + "and its row read an em dash while the core had the band all along")
    expect(item("survey", specifies: false, text: "", band: "585 m").altitudeReading, "585 m",
           "and a pattern over level ground spans nothing, which the core spells as one figure "
           + "rather than as \u{201C}585 m to 585 m\u{201D}")
    expect(item("settings", specifies: false, text: "487 m", band: "0.0 m").altitudeReading,
           "487 m",
           "and a height of its own always wins over a band. The core stopped serving the launch "
           + "row a band in f8e16903b, so nothing produces this pair today -- the assertion is "
           + "what pins the ORDER, which is otherwise unobservable because no item currently "
           + "carries both, and the band-first rule passes every other assertion here")
    expect(item("command", specifies: false, text: "0.0 m", band: "").altitudeReading,
           MissionItem.noAltitude,
           "and an item with neither still reads no altitude")
}

func checkRefusedModesSayWhy() {
    func offer(_ raw: Int, _ title: String, enabled: Bool, reason: String) -> [String: Any] {
        ["raw": raw as NSNumber, "title": title,
         "enabled": enabled as NSNumber, "reason": reason]
    }
    let needsItem = "Add a mission item before choosing how its altitude is measured."
    let listed = AltitudeMode.offers(["modes": [
        offer(1, "Relative To Launch", enabled: true, reason: ""),
        offer(2, "AMSL", enabled: false, reason: needsItem),
        offer(3, "Calculated Above Terrain", enabled: false, reason: needsItem)]])

    expect(listed.count == 3,
           "the fixture carries all three, without which the assertions below pass because the "
           + "list is empty rather than because the rule works")
    expect(AltitudeMode.choosable(listed).count == 1,
           "a refused mode stays out of the picker, because a picker offering a value that "
           + "cannot be set is worse than a shorter one")
    expect(AltitudeMode.refusalNote(listed) ?? "", needsItem,
           "but the core's reason is drawn beside it: two modes vanished from the list and the "
           + "operator was told nothing, while the sentence explaining it was already served")
    expect(AltitudeMode.refusalNote(listed)?.contains("measured. Add") != true,
           "and one sentence is said once, however many modes it refused")

    let allOffered = AltitudeMode.offers(["modes": [offer(1, "Relative To Launch",
                                                          enabled: true, reason: "")]])
    expect(AltitudeMode.refusalNote(allOffered) == nil,
           "nothing refused says nothing at all")
}

func checkCamerasTheOperatorTurnedOn() {
    func camera(_ slot: Int, _ title: String, enabled: Bool,
                configured: Bool, status: String) -> [String: Any] {
        ["slot": slot as NSNumber, "title": title, "status": status,
         "enabled": enabled as NSNumber, "configured": configured as NSNumber]
    }
    let read = VideoStatus(["available": true as NSNumber, "cameras": [
        camera(0, "Camera 1", enabled: true, configured: true, status: "Reconnecting\u{2026}"),
        camera(1, "Camera 2", enabled: true, configured: false, status: "No stream URL"),
        camera(2, "Camera 3", enabled: true, configured: true, status: "Reconnecting\u{2026}"),
        camera(3, "Camera 4", enabled: false, configured: false, status: "No stream URL")]])

    expect(read.cameras.count == 4,
           "the fixture carries all four, without which the assertions below pass because the "
           + "list is empty rather than because the rule works")
    expect(read.listedCameras.map(\.title).joined(separator: ","), "Camera 1,Camera 2,Camera 3",
           "a camera the operator switched on is listed even with no stream URL, because the "
           + "core serves the reason and dropping it left the panel skipping from 1 to 3 with "
           + "nothing to say where 2 had gone")
    expect(read.listedCameras.first { $0.title == "Camera 2" }?.status ?? "", "No stream URL",
           "and it carries the core's own sentence, which is the whole point of listing it")
    expect(!read.listedCameras.contains { $0.title == "Camera 4" },
           "a camera the operator never switched on stays absent: the core says enabled false, "
           + "which is a different answer from configured false and the only one that means "
           + "nothing was asked for")
}

func checkOmittedModesSayWhy() {
    let noTerrain = "This vehicle\u{2019}s firmware cannot hold an altitude above terrain."
    let needsItem = "Add a mission item before choosing how its altitude is measured."
    let read = AltitudeMode.offers([
        "modes": [["raw": 1 as NSNumber, "title": "Relative To Launch",
                   "enabled": true as NSNumber, "reason": ""],
                  ["raw": 2 as NSNumber, "title": "AMSL",
                   "enabled": false as NSNumber, "reason": needsItem]],
        "omitted": [["raw": 4 as NSNumber, "title": "Terrain Frame", "reason": noTerrain]]])

    expect(read.count == 3,
           "a mode the core removed outright is read alongside the ones it served, without which "
           + "every assertion below passes because the list never had it")
    expect(AltitudeMode.choosable(read).map(\.title).joined(separator: ","), "Relative To Launch",
           "an omitted mode carries no enabled flag, so it is false and it stays out of the "
           + "picker -- which is what the core already did by removing it")
    expect(AltitudeMode.refusalNote(read) ?? "", "\(needsItem) \(noTerrain)",
           "but both reasons are drawn now. Terrain Frame vanished with no evidence at all until "
           + "the core began serving omitted, and was legible before that only because "
           + "supportsTerrainFrame happened to ride along beside it")
    expect(AltitudeMode.title(for: 4, in: read), "Terrain Frame",
           "and an omitted mode resolves to its own name rather than to \u{201C}Mode 4\u{201D}")
}

func checkTerrainMarkers() {
    func point(_ x: Double, _ sequence: Int) -> [String: Any] {
        ["x": x as NSNumber, "missionAltitude": 100.0 as NSNumber,
         "terrainAltitude": 50.0 as NSNumber, "sequence": sequence as NSNumber]
    }
    func profile(_ points: [[String: Any]]) -> TerrainProfile {
        TerrainProfile(["points": points, "usable": true as NSNumber])
    }

    let survey = profile([point(0, 0), point(0.54, 2),
                          point(0.73, 3), point(0.8, 3), point(0.9, 3), point(1.0, 3)])
    expect(survey.points.count == 6,
           "the fixture carries the samples the assertions below count, without which every one "
           + "of them compares an empty list against an empty list")
    expect(survey.markers.map(\.label).joined(separator: ","), "0,2,3",
           "one mark per item, in the order the flight reaches them -- a takeoff the vehicle "
           + "flies no leg to contributes no samples and so earns no mark")
    expect(survey.markers.map(\.x) == [0, 0.54, 0.73],
           "each sits at the first sample of its item: a survey collapses to one sequence "
           + "carrying four hundred samples, and marking every one would redraw the strip as a "
           + "solid rule rather than as the boundary between two items")

    expect(survey.labelX(survey.markers[0], width: 400, inset: 8) == 8,
           "the number for item 0 is pulled inside the plot: centred on its own tick at x=0 it "
           + "rendered half outside and lost its left half every time")
    expect(survey.labelX(survey.markers[1], width: 400, inset: 8) == 216,
           "and a mark with room either side is not moved at all")

    let climb = profile([point(0, 0), point(0, 1), point(0.54, 2)])
    expect(climb.markers.map(\.label).joined(separator: ","), "0\u{2013}1,2",
           "an ArduPilot takeoff climbs straight up, so the core gives it the launch point's own "
           + "place on the axis -- two items at one x are one mark, and drawing both put their "
           + "numbers on top of each other and hid the takeoff entirely")
    expect(climb.markers.count == 2,
           "and they are one mark, not two at the same place")

    let unstamped = profile([point(0, 0), ["x": 0.5 as NSNumber,
                                           "missionAltitude": 100.0 as NSNumber]])
    expect(unstamped.markers.map(\.label) == ["0"],
           "a sample the core did not stamp is drawn in the line and marked nowhere, rather than "
           + "collecting under a sequence of zero that would put a second mark on item 0")
}

func checkWithheldLegFigures() {
    let withheld = MissionItem(view: ["index": 2 as NSNumber, "sequence": 2 as NSNumber,
                                      "name": "Waypoint", "kind": "waypoint",
                                      "flownLeg": true as NSNumber,
                                      "coordinate": ["latitude": -35.0 as NSNumber,
                                                     "longitude": 149.0 as NSNumber]],
                               selected: -1)
    expect(withheld.hasPosition,
           "the fixture is a placed item, so the leg row would be drawn for it and the readings "
           + "below are the ones an operator would see")
    expect(withheld.distanceText, MissionItem.noAltitude,
           "the core withholds the four leg figures until its walk has run, because zero is a "
           + "real distance and zero degrees is due north -- an empty string drew three blank "
           + "rows, where the em dash says the figure is not known yet")
    expect(withheld.azimuthText, MissionItem.noAltitude, "and the heading")
    expect(withheld.altitudeChangeText, MissionItem.noAltitude, "and the altitude change")
}

func checkLegsSpelled() {
    func item(_ index: Int, _ kind: String, flown: Bool = true, ends: Bool = false,
              azimuth: String = "0\u{00B0}", distance: String = "0 m") -> MissionItem {
        MissionItem(view: ["index": index as NSNumber, "sequence": index as NSNumber,
                           "name": kind, "kind": kind,
                           "endsRoute": ends as NSNumber, "flownLeg": flown as NSNumber,
                           "azimuthText": azimuth, "distanceText": distance,
                           "altitudeChangeText": "+0.0 m",
                           "coordinate": ["latitude": -35.0 as NSNumber,
                                          "longitude": 149.0 as NSNumber]], selected: -1)
    }

    let start = item(0, "settings")
    let takeoff = item(1, "takeoff", flown: false)
    let reached = item(2, "waypoint", azimuth: "60\u{00B0}", distance: "14.11 km")
    let plan = [start, takeoff, reached]

    expect(MissionItem.legs(plan) == [2],
           "only the waypoint was reached by a leg: nothing flies to where the route begins, and "
           + "a takeoff the vehicle does not fly a leg to earns no figures either")

    let ended = plan + [item(3, "command", flown: false, ends: true),
                        item(4, "waypoint")]
    expect(MissionItem.legs(ended) == [2],
           "a waypoint after the return to launch is uploaded and never reached, so it has no leg "
           + "-- the same routeEnd rule the list and the map already share")

    let dueNorth = [start, item(1, "waypoint", azimuth: "0\u{00B0}", distance: "250 m")]
    expect(MissionItem.legs(dueNorth) == [1],
           "a leg flown due north reads 0 degrees, which is why the rule is the route's and not "
           + "the figures': reading the core's zero as absence would hide this leg entirely")

    expect(reached.distanceText, "14.11 km",
           "and the core's own sentence is carried through, not reformatted here")
}

func checkClearanceSentence() {
    func profile(_ extra: [String: Any]) -> TerrainProfile {
        TerrainProfile(["usable": true as NSNumber, "groundKnown": true as NSNumber,
                        "points": []].merging(extra) { _, new in new })
    }
    let intruding = profile(["hasCollision": true as NSNumber, "clearanceComplete": true as NSNumber,
                             "minClearanceMetres": -168.0 as NSNumber, "clearanceText": "168 m"])
    expect(intruding.clearanceSentence, "Mission is below terrain by up to 168 m",
           "the operator's next move after a collision is to pick a new altitude, and the only "
           + "instrument was eyeballing a line against a fill on a plot 90 points tall spanning "
           + "305 m -- 3.4 m per point. Measured on the running app, that mission ran 168 m under")
    expect(intruding.showsClearance, "and a collision always says its depth")

    let clearing = profile(["hasCollision": false as NSNumber, "clearanceComplete": true as NSNumber,
                            "minClearanceMetres": 42.0 as NSNumber, "clearanceText": "42 m"])
    expect(clearing.clearanceSentence, "Clears terrain by 42 m",
           "the core signs it -- headroom above, intrusion below -- so one sentence serves both "
           + "and neither head decides which way round the subtraction goes")
    expect(clearing.showsClearance, "and a margin is worth knowing before flying, not only a fault")

    let partial = profile(["hasCollision": false as NSNumber, "clearanceComplete": false as NSNumber,
                           "unknownTerrain": 3 as NSNumber,
                           "minClearanceMetres": 42.0 as NSNumber, "clearanceText": "42 m"])
    expect(!partial.showsClearance,
           "but not when the core says the measurement is incomplete: the smallest clearance it "
           + "could take is not the smallest there is, and a reassuring number is worse than none. "
           + "The head worked this out from the unknown count until the core began answering it")

    let intrudingPartly = profile(["hasCollision": true as NSNumber,
                                   "clearanceComplete": false as NSNumber,
                                   "unknownTerrain": 3 as NSNumber,
                                   "minClearanceMetres": -90.0 as NSNumber, "clearanceText": "90 m"])
    expect(intrudingPartly.showsClearance,
           "a collision states its depth even with ground missing elsewhere -- being under the "
           + "ground somewhere is not made uncertain by not knowing the rest, and the depth found "
           + "is a floor on the depth there is")

    let unmeasured = profile(["hasCollision": false as NSNumber,
                              "clearanceComplete": false as NSNumber,
                              "unknownTerrain": 0 as NSNumber])
    expect(!unmeasured.showsClearance,
           "nothing measured is not the same as nothing in the way, and the core is what knows "
           + "which of those it found")
    expect(unmeasured.clearanceSentence, "",
           "and with nothing measured and nothing struck there is no sentence to say -- this "
           + "used to fall back to the collision wording and rely on showsClearance never being "
           + "true here, which held only as long as the core paired complete with a magnitude")

    let silent = profile(["hasCollision": true as NSNumber, "clearanceComplete": false as NSNumber])
    expect(silent.clearanceSentence, "Mission is below terrain",
           "a collision the core could not put a depth on still says it collides, rather than "
           + "reading as a sentence with its number dropped out")

    let completeWithoutFigure = profile(["hasCollision": false as NSNumber,
                                         "clearanceComplete": true as NSNumber,
                                         "clearanceText": ""])
    expect(completeWithoutFigure.clearanceSentence != TerrainProfile.below,
           "a mission that did not collide is never given the collision wording, whatever the "
           + "core paired with it")
    expect(!completeWithoutFigure.showsClearance,
           "and a clearance with no magnitude to state is not a sentence worth drawing")

}

func checkCollisionRuns() {
    func profile(_ flags: [Bool]) -> TerrainProfile {
        TerrainProfile(["usable": true as NSNumber, "groundKnown": true as NSNumber,
                        "points": flags.enumerated().map { index, hit in
                            ["x": Double(index) / Double(max(flags.count - 1, 1)) as NSNumber,
                             "missionAltitude": 100.0 as NSNumber,
                             "terrainAltitude": 50.0 as NSNumber,
                             "collision": hit as NSNumber]
                        }])
    }
    func spans(_ flags: [Bool]) -> String {
        profile(flags).collisionRuns
            .map { String(format: "%.2f-%.2f", $0.lowerBound, $0.upperBound) }
            .joined(separator: ",")
    }

    expect(spans([false, true, true, true, false]), "0.25-0.75",
           "one unbroken stretch is one span. Measured on the running app: a survey's 407 samples "
           + "gave 405 colliding points in a single run, drawn as 405 overlapping dots each "
           + "covering six of its neighbours")
    expect(spans([true, true, false, false, true]), "0.00-0.25,1.00-1.00",
           "two separate stretches stay two, which is the whole reason not to draw a dot per "
           + "sample -- smeared together they read as one region of ground to clear")
    expect(spans([false, false, false]), "", "a mission that clears the ground marks nothing")
    expect(spans([true, true, true]), "0.00-1.00", "and one that never clears it is a single span")
    expect(spans([]), "", "an empty profile has nothing to mark rather than trapping")
}

func checkSurveyWatch() {
    expect(SurveyWatch.signals(2).joined(separator: ","),
           "plan.missionController.visualItems.2.cameraShots,"
           + "plan.missionController.visualItems.2.complexDistance",
           "a survey answers its area and shot interval as soon as it has a polygon, and its shot "
           + "COUNT and flown DISTANCE only once the transects are computed -- after the read that "
           + "drew the panel. Measured: an em-dash for both while the core answered 1043 shots "
           + "over 7047 m, and it stayed that way until something forced a reload")
    expect(SurveyWatch.signals(survey: nil).isEmpty,
           "and nothing is watched while no survey is selected, because the panel has no survey "
           + "to be stale about")
    expect(SurveyWatch.signals(survey: 2) == SurveyWatch.signals(2),
           "a selected survey asks for exactly what that index asks for")
    expect(SurveyWatch.properties.joined(separator: ","), "cameraShots,complexDistance",
           "these two names are interpolated into bridge paths, where a misspelling is a signal "
           + "that never arrives rather than an error. interpolated-names.py pins them, and they "
           + "span two headers -- the shot count is on the transect class, the distance on the "
           + "complex class above it")
}

func checkRemoveOutcome() {
    expect(RemoveOutcome(["ok": true as NSNumber, "removed": 2 as NSNumber,
                          "remaining": 3 as NSNumber]) == .removed,
           "the core counts the plan before and after, so a removal that took is one the head was "
           + "told about. This head used to invoke removeVisualItem and never look at the answer")

    expect(RemoveOutcome(["ok": false as NSNumber,
                          "reason": "The first entry holds the plan's own settings and cannot be "
                          + "removed."]) == .refused("The first entry holds the plan's own "
                          + "settings and cannot be removed."),
           "and a refusal arrives with the controller's own sentence rather than silence. The "
           + "head's gate asked whether the SEQUENCE was past the first where the core refuses on "
           + "the INDEX -- two schemes that agree only because the settings entry is both")

    expect(RemoveOutcome(["ok": false as NSNumber, "reason": ""]) == .refused(RemoveOutcome.refusedWithoutReason),
           "a refusal with nothing said still reads as a refusal, because an item still on screen "
           + "with no explanation is what this replaced")
    expect(RemoveOutcome([:]) == .refused(RemoveOutcome.refusedWithoutReason),
           "and an answer that did not decode is a refusal too: a removal is a write, so the "
           + "direction that fails closed is the one that leaves the plan as it was")
    expect(RemoveOutcome(["ok": true as NSNumber]) == .removed,
           "the count the core reports alongside is not decoded here, because reload re-reads the "
           + "plan anyway and a field only a test ever reads is not a mechanism")
}

func checkBridgeWatchers() {
    var registry = BridgeWatchers()
    registry.set("plan", ["view.missionSummary"])
    registry.set("fly", ["view.missionSummary", "view.flyState"])

    expect(registry.clients(watching: "view.missionSummary").joined(separator: ","), "fly,plan",
           "two clients watching one path both hear about it. The Plan window and the Fly view "
           + "each hold their own MissionStore, and a fixed client name would have the second "
           + "silently unsubscribe the first -- the core keys its watch list by client")
    expect(registry.clients(watching: "view.flyState").joined(separator: ","), "fly",
           "and a path only one of them asked for reaches only that one")
    expect(registry.clients(watching: "view.plan").isEmpty,
           "an event for a path nobody asked for goes nowhere, rather than to everybody")

    expect(registry.csv("fly"), "view.missionSummary,view.flyState",
           "the paths reach the core comma separated in the order asked. The core splits on "
           + "commas at paren depth zero, so a parameterised path keeps its own commas")
    registry.set("seed", ["view.missionSeed(survey,47,8)", "view.plan"])
    expect(registry.csv("seed"), "view.missionSeed(survey,47,8),view.plan",
           "which is why this head joins and never escapes -- a path carrying two commas of its "
           + "own survives the trip")

    registry.set("fly", [])
    expect(registry.clients(watching: "view.missionSummary").joined(separator: ","), "plan",
           "a client that stops watching stops hearing, and the other one carries on. A window "
           + "closing must not take the other window's updates with it")
    expect(registry.csv("fly"), "",
           "and it asks the core for nothing, which is how the core drops it from the list")
}

func checkBlockedItems() {
    let ok = MissionItem(view: ["index": 2, "sequence": 2, "name": "Waypoint",
                                "blocked": false, "awaitingTerrain": false], selected: -1)
    expect(!ok.blocked, "an item the controller calls ready does not block the plan")
    expect(ok.blockedReason == nil, "and carries no reason to show")

    let unset = MissionItem(view: ["index": 1, "sequence": 1, "name": "Takeoff",
                                   "blocked": true, "awaitingTerrain": false,
                                   "blockedReason": "Set its location"], selected: -1)
    expect(unset.blocked,
           "a takeoff whose location was never set blocks the plan. Measured on the running app: "
           + "it reads state 2 and \"Set its location\", its coordinate is 0,0, and nothing in "
           + "this window said so -- the position column showed a dash and the operator was left "
           + "to work out why Save stayed refused")
    expect(unset.blockedReason ?? "", "Set its location",
           "and the controller's own sentence is what gets shown, not one invented here")

    let silent = MissionItem(view: ["index": 1, "sequence": 1, "name": "Takeoff",
                                    "blocked": true, "awaitingTerrain": false,
                                    "blockedReason": ""], selected: -1)
    expect(silent.blocked && silent.blockedReason == nil,
           "an item that blocks without saying why still blocks -- the row cannot explain it, "
           + "but the plan is no more saveable for the silence")

    let terrain = MissionItem(view: ["index": 3, "sequence": 3, "name": "Waypoint",
                                     "blocked": false, "awaitingTerrain": true,
                                     "blockedReason": "Waiting for terrain"], selected: -1)
    expect(!ok.unready,
           "an item the controller calls ready wears no amber seal -- pinning unready to a "
           + "constant true fired nothing, so a plan where every row wore the seal and the "
           + "banner never cleared would have passed")
    expect(!terrain.blocked && terrain.awaitingTerrain && terrain.unready,
           "an item waiting on terrain heights is unready too, but it is a wait on a server "
           + "and not a task for the operator. QGC keeps the two apart -- NotReadyForSaveData "
           + "lists the incomplete items and selects the next one, NotReadyForSaveTerrain shows "
           + "a different message with no item list because there is nothing to select")
    expect(!unset.awaitingTerrain && unset.blocked && unset.unready,
           "and an item the operator has to finish is not a terrain wait, though both leave the "
           + "plan unready. Neither stops a SAVE: ce67911a1 measured an eight-item plan with a "
           + "half-drawn takeoff writing 92943 bytes while the menu called it unsaveable, which "
           + "is why this property is no longer called stopsSave -- it stops nothing, it colours "
           + "a seal, and the name asserted a behaviour the fix had already disproved")

    let far = MissionItem(view: ["index": 7, "sequence": 210, "name": "Corridor Scan",
                                 "simple": false, "kind": "corridor"], selected: -1)
    expect(far.canRemove,
           "MissionController::removeVisualItem guards on viIndex <= 0 -- the INDEX, which is "
           + "also what mission.remove is passed. In the every-kind plan seven of ten items "
           + "carry a sequence different from their index, the corridor sitting at index 7 with "
           + "sequence 210. THIS ASSERTION DOES NOT SEPARATE THE TWO RULES and is not claimed "
           + "to: 210 > 0 as surely as 7 > 0, and reverting to sequence > 0 passes every check "
           + "in this file. No reachable state separates them, because exactly one item carries "
           + "sequence 0 and it is also index 0. The change aligns the gate with the key the "
           + "operation uses; it fixes no behaviour and the mutation says so")
    let launchRow = MissionItem(view: ["index": 0, "sequence": 0, "name": "Launch",
                                       "simple": false, "kind": "settings"], selected: -1)
    expect(!launchRow.canRemove && !launchRow.canChangeCommand,
           "and the one row the controller refuses to remove is still refused by both gates")
    let takeoff = MissionItem(view: ["index": 1, "sequence": 1, "name": "Takeoff",
                                     "simple": true, "kind": "takeoff"], selected: -1)
    expect(takeoff.canRemove && !takeoff.canChangeCommand,
           "the takeoff is removable and NOT re-commandable, which is why canRemove and "
           + "canChangeCommand cannot share a rule: removeVisualItem permits index 1, while "
           + "changing a takeoff's command would leave a plan whose launch is a plain waypoint")

    let absent = MissionItem(view: ["index": 1, "sequence": 1], selected: -1)
    expect(!absent.blocked,
           "and an item whose state the controller did not report does NOT block. Every other "
           + "item would decode to blocked on one missing key, and a plan that refuses to save "
           + "with a row of unexplained warnings is worse than one that lets the controller "
           + "refuse the save itself, which it still does")
}

func checkBlockedBanner() {
    func item(_ seq: Int, _ name: String, _ state: Int, _ message: String = "") -> MissionItem {
        MissionItem(view: ["index": seq, "sequence": seq, "name": name,
                           "blocked": state == 2, "awaitingTerrain": state == 1,
                           "blockedReason": message], selected: -1)
    }
    let general = "An item is still being drawn, so the plan cannot be saved or sent."
    let one = [item(0, "Mission Start", 0), item(1, "Takeoff", 2, "Set its location"),
               item(2, "Waypoint", 0)]
    expect(MissionItem.blockedBanner(one, reason: general), "Takeoff (1): set its location",
           "with exactly one item at fault the banner names it. The banner and the row used to "
           + "give different accounts of the same fault -- \"an item is still being drawn\" "
           + "against \"Set its location\" -- and an operator reading top to bottom went looking "
           + "for a half drawn polygon")
    expect(MissionItem.blockedItem(one)?.sequence == 1,
           "and the banner becomes a button that selects that row, so a blocked item in a thirty "
           + "item survey is not found by eye")

    let several = [item(1, "Takeoff", 2, "Set its location"),
                   item(2, "Survey", 2, "Draw its area")]
    expect(MissionItem.blockedBanner(several, reason: general), general,
           "with several at fault the plan's own sentence stands, because naming one of them "
           + "would tell the operator to fix that row and find the save still refused")
    expect(MissionItem.blockedItem(several) == nil, "and there is nowhere for a button to go")

    let waiting = [item(1, "Takeoff", 0), item(3, "Waypoint", 1, "Waiting for terrain")]
    expect(MissionItem.blockedBanner(waiting, reason: general), general,
           "a single item waiting on terrain does NOT get named. The plan's own sentence already "
           + "says the plan waits on terrain heights, and naming a row invites the operator to "
           + "go and fix something that no click of theirs can fix")
    expect(MissionItem.blockedItem(waiting) == nil,
           "and the banner is not a button, because there is nothing on that row to do")

    let clean = [item(1, "Takeoff", 0), item(2, "Waypoint", 0)]
    expect(MissionItem.blockedBanner(clean, reason: general), general,
           "a plan whose items are all ready keeps whatever reason the plan itself gave -- the "
           + "refusal can be about the plan rather than about any one item")

    let mute = [item(1, "Takeoff", 2, "")]
    expect(MissionItem.blockedBanner(mute, reason: general), general,
           "and an item that blocks without saying why cannot name itself usefully, so the "
           + "general sentence is still the honest one")

    expect("Set its location".lowercasedFirst, "set its location",
           "the controller writes each message as its own sentence, so it needs lowering to sit "
           + "after a colon")
    expect("".lowercasedFirst, "", "and an empty one stays empty rather than trapping")
}

func checkMavlinkConsole() {
    let read = MavlinkConsole(["lines": ["nsh> ver all", "FW git-hash: 1a2b3c", ""]],
                              connected: true)
    expect(read.lines.joined(separator: "|"), "nsh> ver all|FW git-hash: 1a2b3c",
           "MAVLinkConsoleController keeps the line still being assembled as the last entry, and "
           + "it is empty until a newline arrives; drawing it adds a blank row that appears and "
           + "disappears as characters land")
    expect(read.last ?? "", "FW git-hash: 1a2b3c",
           "the tail is what a console is for, so the head names it rather than making the "
           + "operator scroll a long session to find what just happened")
    expect(read.describes, "and a console with output describes something")

    expect(MavlinkConsole(["lines": ["still typing"]], connected: true)
        .lines.joined(separator: "|"), "still typing",
           "but a partial line that is NOT empty is real output and is kept -- the tell is the "
           + "empty string, not the position")

    expect(MavlinkConsole.none.copyable, "",
           "an empty console copies an empty string rather than trapping on a missing last line")
    expect(MavlinkConsole([:], connected: true).describes == false,
           "an empty read describes nothing rather than an empty console")
    expect(MavlinkConsole([:], connected: true).emptyText,
           "Nothing yet. The shell answers commands, and those are sent from the Qt build.",
           "and a connected vehicle that has said nothing is told apart from no vehicle at all -- "
           + "and told where the commands go, because shell output only answers a command and "
           + "this page cannot send one, so \"nothing yet\" alone counsels an endless wait")
    expect(read.copyable, "nsh> ver all\nFW git-hash: 1a2b3c",
           "the whole buffer is copyable in one piece. SwiftUI selection does not span sibling "
           + "Text views, so per-line selection cannot lift a boot log into a bug report, which "
           + "is the normal next step after reading one")
    expect(MavlinkConsole.none.emptyText, "Connect a vehicle to see its console output.",
           "because those are different situations and only one is worth waiting through")

    expect(MavlinkConsole(["lines": ["a", 7 as NSNumber, "b"]], connected: true)
        .lines.joined(separator: "|"), "a|b",
           "a line that is not a string is dropped rather than rendered as its description")

    expect(MavlinkConsole.readOnlyNotice.contains("Commands are sent from the Qt build"),
           "the page says it only reads. sendCommand puts a string on the vehicle's shell, which "
           + "can reboot it, rewrite its parameters or arm it, and nothing here can send one to "
           + "check the wiring -- so the input is unbuilt and the page says so rather than "
           + "offering a field that silently does nothing")
}

func checkModeSlots() {
    func slot(_ n: Int, _ mode: Any, _ live: Bool) -> [String: Any] {
        ["slot": n as NSNumber, "mode": mode, "live": live as NSNumber]
    }
    let read = ModeSlots(["available": true as NSNumber, "channel": 5 as NSNumber,
                          "liveSlot": 3 as NSNumber, "reason": "",
                          "slots": [slot(1, "Stabilize", false), slot(2, "Loiter", false),
                                    slot(3, "Loiter", true), slot(4, NSNull(), false)]])
    expect(read.slots.count == 4, "every position the core reports is carried across")
    expect(read.isLive(3), "the live position is the one the transmitter is selecting")
    expect(!read.isLive(2),
           "and a position holding the SAME mode as the live one is not live. This head used to "
           + "mark a position active when its configured mode equalled the vehicle's current "
           + "mode, so two positions set to Loiter both lit up -- and a mode chosen from the "
           + "ground station lit whichever position held it while the switch sat elsewhere")
    expect(read.slots[3].mode == nil, "a position with no mode set reads as none, not as empty text")

    let unreachable = ModeSlots(["available": true as NSNumber, "liveSlot": 0 as NSNumber,
                                 "reason": "The transmitter is not sending on the mode channel.",
                                 "slots": [slot(1, "Stabilize", false)]])
    expect(!unreachable.isLive(1),
           "with no live position nothing is marked, because showing the wrong switch position "
           + "is worse than showing none")
    expect(unreachable.reason, "The transmitter is not sending on the mode channel.",
           "and the core says why rather than the screen going quiet")

    expect(!ModeSlots(["available": false as NSNumber, "liveSlot": 2 as NSNumber,
                       "slots": [slot(2, "Loiter", true)]]).isLive(2),
           "an unavailable read marks nothing even when it carries a live slot, because the two "
           + "have to agree before this head paints a position green")
    expect(ModeSlots([:]).describes == false, "an empty read describes nothing")
    expect(ModeSlots.none.reason, "", "and says nothing rather than inventing a reason")
}

func checkVideoFrame() {
    let frame = VideoFrame(width: 1920, height: 1080, stride: 7808)
    expect(frame?.byteCount == 7808 * 1080,
           "a frame is as big as its stride times its height, not its width times its height -- "
           + "the decoder pads rows and reading width * height falls short of the last row")
    expect(frame?.fits(7808 * 1080) == true, "and it fits a buffer of exactly that size")
    expect(frame?.fits(7808 * 1080 - 1) == false, "but not one a single byte short")
    expect(frame?.byteCount != 1920 * 4 * 1080,
           "and the padded frame is measurably bigger than an unpadded one, so this fixture can "
           + "tell the two formulas apart")

    expect(VideoFrame(width: 1920, height: 1080, stride: 7679) == nil,
           "a stride narrower than the pixels it claims to carry is not a row of them. This runs "
           + "thirty times a second against a buffer the C side filled, so a geometry taken on "
           + "trust is a read past the end of it")
    expect(VideoFrame(width: 0, height: 1080, stride: 7680) == nil, "nor is a frame with no width")
    expect(VideoFrame(width: 1920, height: 0, stride: 7680) == nil, "nor one with no height")
    expect(VideoFrame(width: 1920, height: 1080, stride: 0) == nil, "nor one with no stride")
    expect(VideoFrame(width: -1920, height: 1080, stride: 7680) == nil,
           "and a negative dimension is refused before it becomes an Int, because the C ABI "
           + "reports these as Int32 and a negative one multiplied out is a very large positive")

    expect(VideoFrame.capacity(width: 1920, height: 1080) == 1920 * 4 * 1080,
           "a width whose row is already aligned asks for exactly its own pixels")
    expect(VideoFrame(width: 1918, height: 1080, stride: 7680)?
        .fits(VideoFrame.capacity(width: 1918, height: 1080)) == true,
           "and a width whose rows the decoder pads still fits, because the buffer is sized for "
           + "a padded row. Asking for width * height * 4 left it short for every padded width, "
           + "and a short buffer is a dropped frame and a black panel with nothing said -- 1920 "
           + "hides it because 7680 is already aligned")
    expect(VideoFrame.capacity(width: 0, height: 1080) == 0,
           "and no buffer at all before the first frame reports a size, so nothing is allocated "
           + "on the thirty times a second that run before a stream arrives")
}
