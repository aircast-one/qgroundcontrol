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
    "units": "", "readOnly": false as NSNumber, "rebootRequired": false as NSNumber,
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
let vibration = VibrationReading(vibrationJson)
expect(vibration.axes.count == 3, "each axis the core reports is carried across")
expect(vibration.axes.map(\.label).joined(separator: ","), "X,Y,Z",
       "with the core's display label, so no head upper-cases an axis name itself")
expect(vibration.worst == .danger,
       "one bad axis makes the whole reading dangerous, and the core decides which is worst")
expect(vibration.axes[1].severity == .danger, "each bar takes its own colour from its own severity")
expect(vibration.dangerLevel == 60 && vibration.warningLevel == 30 && vibration.scaleMaximum == 90,
       "the thresholds the bars and the scale are drawn against come from the core, which is "
       + "where ArduPilot and PX4's flight guidance now lives rather than in two heads")
expect(vibration.clipping, "any clip count above zero is clipping")

let quiet = VibrationReading(["available": false as NSNumber,
                              "axes": [["axis": "x", "label": "X"]]])
expect(quiet.worst == nil,
       "a vehicle reporting no level has no worst level, which is not the same as normal")
expect(quiet.axes[0].value == nil && quiet.axes[0].severity == nil,
       "and an axis with no reading draws no bar rather than a bar at zero, which would look "
       + "like a perfectly still airframe")
expect(!VibrationReading.unavailable.available, "the unavailable reading reports itself as such")
expect(VibrationReading.Severity("molten") == nil,
       "a severity this head does not know is no severity, not the reassuring one")

// QGC appends "Unknown: N" to enumStrings when a value is outside the enum and points
// enumIndex at it; showing that instead of the number is a regression in readability.
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

// Page selection is probe-driven because a locked screen cannot deliver a sidebar
// click; it must reject a page that does not exist rather than blanking the window.
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

// The PWM bands are firmware constants, not parameters: an operator matching a
// transmitter switch to a mode cannot see them anywhere else.
expect(String(FlightModePosition.all.count), "6", "ArduPilot maps six switch positions")
expect(FlightModePosition.all.first!.pwmRange, "up to 1230", "the first band is open-ended below")
expect(FlightModePosition.all.last!.pwmRange, "1750 and above", "the last band is open-ended above")
expect(FlightModePosition.all.map(\.parameter).joined(separator: ","),
       "FLTMODE1,FLTMODE2,FLTMODE3,FLTMODE4,FLTMODE5,FLTMODE6", "positions map to FLTMODE1..6 in order")

// A firmware that names them differently gets nothing rather than six broken rows.
expect(FlightModePosition.present(in: ["FLTMODE1", "FLTMODE3"]).map(\.index).map(String.init).joined(separator: ","),
       "1,3", "only reported positions appear")
expect(FlightModePosition.present(in: []).isEmpty, "a vehicle without them shows no positions")

let waypoint = MissionItem(json: [
    "sequenceNumber": 1, "commandName": "Waypoint", "isCurrentItem": false,
    "coordinate": ["latitude": -35.3629, "longitude": 149.165, "altitude": NSNull()],
    "facts": [["name": "Altitude", "value": 50.0, "units": "ft"]],
    "specifiesAltitude": true], index: 1)
expect(waypoint.altitudeText, "50.0 ft",
       "altitude comes from the fact, and so does the unit beside it")
expect(waypoint.positionText, "-35.362900, 149.165000", "position formats to six decimals")
expect(waypoint.hasPosition, "a waypoint with a coordinate has a position")

let start = MissionItem(json: [
    "sequenceNumber": 0, "commandName": "Mission Start", "isCurrentItem": true,
    "coordinate": ["latitude": -35.36, "longitude": 149.16, "altitude": 584.09],
    "facts": []], index: 0)
expect(start.altitudeText, "584 m", "falls back to the coordinate altitude")

let startFact = MissionItem(json: [
    "sequenceNumber": 0, "commandName": "Mission Start",
    "coordinate": ["latitude": -35.36, "longitude": 149.16, "altitude": 584.09],
    "facts": [["name": "PlannedHomePositionAltitude", "value": 1916.3, "units": "ft"]]], index: 0)
expect(startFact.altitudeText, "1916 ft",
       "but mission start has its own altitude fact, and the row must not disagree with the field below it")

let startInFeet = MissionItem(json: [
    "sequenceNumber": 0, "commandName": "Mission Start",
    "coordinate": ["latitude": -35.36, "longitude": 149.16, "altitude": 584.09],
    "facts": []], index: 0, verticalMeasure: Measure(units: "ft", factor: 3.28084))
expect(startInFeet.altitudeText, "1916 ft",
       "and when only the coordinate is left, its metres are converted rather than relabelled")
expect(start.isCurrent, "the current item is flagged")
expect(waypoint.index == 1, "an item remembers the list position its bridge path needs")
expect(waypoint.specifiesAltitude, "a waypoint's altitude is editable")
expect(!start.specifiesAltitude, "an item that does not specify altitude is not editable")

let bare = MissionItem(json: ["sequenceNumber": 2, "commandName": "Delay", "facts": []], index: 2)
expect(!bare.hasPosition, "an item without a coordinate has no position")
expect(bare.positionText, "—", "and shows nothing rather than a false one")
expect(bare.altitudeText, "—", "same for altitude")

func checkMapFraming() {
    let canberra = MapFrame(latitudes: [-35.363262, -35.362900],
                            longitudes: [149.165237, 149.165000])
    expect(abs(canberra.centreLatitude - -35.363081) < 1e-5, "mission centre sits between the waypoints")
    expect(abs(canberra.centreLongitude - 149.1651185) < 1e-5, "mission centre longitude sits between the waypoints")
    expect(canberra.latitudeDelta < 0.01, "two close waypoints frame tightly, not to a hemisphere")
    expect(canberra.isUsable, "a real mission frames to a usable region")

    let single = MapFrame(latitudes: [-35.36], longitudes: [149.16])
    expect(single.latitudeDelta == MapFrame.minimumDelta, "one waypoint still gets a minimum span")
    expect(single.longitudeDelta == MapFrame.minimumDelta, "one waypoint still gets a minimum longitude span")
    expect(abs(single.centreLatitude - -35.36) < 1e-9, "and is centred on that waypoint")
    expect(abs(single.centreLongitude - 149.16) < 1e-9, "on both axes")
    expect(single.isUsable, "a single point is a usable frame, not a reason to show the whole world")

    let empty = MapFrame(latitudes: [], longitudes: [])
    expect(empty.isUsable, "no waypoints yields a usable region rather than NaN")

    let bogus = MapFrame(latitudes: [-35.36, .nan, 1000], longitudes: [149.16, .infinity])
    expect(abs(bogus.centreLatitude - -35.36) < 1e-9, "a NaN or out-of-range latitude cannot drag the frame")
    expect(abs(bogus.centreLongitude - 149.16) < 1e-9, "a non-finite longitude cannot drag the frame")
    expect(bogus.isUsable, "a bogus coordinate still leaves a usable region")
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
}
checkFenceGeometry()

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

    let ready = VehicleReadiness(["ready": true as NSNumber, "headline": "Ready to fly",
                                  "detail": "Setup complete and all enabled sensors are healthy."])
    expect(ready.ready && ready.headline == "Ready to fly",
           "the summary carries the core's verdict rather than recomputing it")

    let faulty = VehicleReadiness(["ready": false as NSNumber,
                                   "headline": "2 sensors reporting a fault",
                                   "detail": "GPS, Pre-Arm Check"])
    expect(!faulty.ready && faulty.detail == "GPS, Pre-Arm Check",
           "including the sensor faults, which the core now folds in itself, so this summary no "
           + "longer has to be handed the sensor list by a store that may not have loaded")

    expect(VehicleReadiness([:]) == VehicleReadiness(ready: false, headline: "", detail: ""),
           "an empty answer is not ready and says nothing, rather than claiming readiness")
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
    let home = MissionItem(json: ["sequenceNumber": 0, "commandName": "Mission Start"], index: 0)
    let waypoint = MissionItem(json: ["sequenceNumber": 1, "commandName": "Waypoint"], index: 1)
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

    let waypoint = MissionItem(json: [
        "sequenceNumber": 1, "commandName": "Waypoint", "isSimpleItem": true,
    ], index: 1)
    let home = MissionItem(json: [
        "sequenceNumber": 0, "commandName": "Mission Start", "isSimpleItem": true,
    ], index: 0)
    let survey = MissionItem(json: [
        "sequenceNumber": 2, "commandName": "Survey", "isSimpleItem": false,
    ], index: 2)
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
    let cameraFacts = ItemFact.camera([
        ["name": "SensorWidth", "property": "sensorWidth", "valueString": "7.6", "units": "mm"],
        ["name": "FrontalOverlap", "property": "frontalOverlap", "valueString": "70", "units": "%"],
        ["name": "DistanceToSurface", "property": "distanceToSurface", "valueString": "50", "units": "m"],
        ["name": "SideOverlap", "property": "sideOverlap", "valueString": "70", "units": "%"],
        ["name": "ImageDensity", "property": "imageDensity", "valueString": "1.8", "units": "cm/px"],
    ], label: { "<\($0)>" })
    expect(cameraFacts.count == 4, "only the four that decide a survey are offered")
    expect(cameraFacts[0].id, "cameraCalc.distanceToSurface", "a camera fact is addressed through cameraCalc")
    expect(cameraFacts[0].group, ItemFact.cameraGroup, "and is grouped as camera")
    expect(!cameraFacts.contains { $0.name == "SensorWidth" }, "camera hardware specs are not survey settings")
    expect(cameraFacts.map(\.name).joined(separator: ","), ["DistanceToSurface", "ImageDensity", "FrontalOverlap", "SideOverlap"].joined(separator: ","),
           "they read in the order an operator thinks about them")

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
    let delay = MissionItem(json: [
        "sequenceNumber": 1, "commandName": "Delay", "isSimpleItem": true,
        "specifiesCoordinate": false,
        "coordinate": ["latitude": 0, "longitude": 0],
    ], index: 1)
    expect(!delay.hasPosition, "a command with no position of its own is not placed at 0,0")
    expect(delay.positionText, "\u{2014}", "and shows nothing rather than the Gulf of Guinea")

    let waypoint = MissionItem(json: [
        "sequenceNumber": 2, "commandName": "Waypoint", "specifiesCoordinate": true,
        "coordinate": ["latitude": -35.363, "longitude": 149.165],
    ], index: 2)
    expect(waypoint.hasPosition, "a waypoint with a real coordinate is placed")

    let nullIsland = MissionItem(json: [
        "sequenceNumber": 3, "commandName": "Waypoint", "specifiesCoordinate": true,
        "coordinate": ["latitude": 0, "longitude": 0],
    ], index: 3)
    expect(!nullIsland.hasPosition, "an unset coordinate is not a position even when the command wants one")
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
    expect(FlyDetail.link(rcRSSI: 0, localRSSI: nil, remoteRSSI: nil).isEmpty,
           "and a zero RC reading is absence, not a dead stick")

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

    expect(ItemSpeed.factName != ItemSpeed.property,
           "the fact is named FlightSpeed and the property is flightSpeed; reading and writing use different keys")

    expect(!ItemSpeed(json: [:]).available,
           "an item with no speed section offers nothing rather than a dead control")
    expect(ItemSpeed(json: ["available": true, "specifyFlightSpeed": true]).value == nil,
           "and one whose speed fact is missing shows no number rather than a zero")
}

checkItemSpeed()
checkMissionStartSpeed()
checkLaunchPosition()
checkLaunchAltitudeIsNotListedTwice()

func checkTelemetryUnits() {
    var feet = FlyTelemetry()
    feet.altitude = 1916.2
    feet.groundSpeed = 12.5
    feet.distanceUnits = "ft"
    feet.speedUnits = "ft/s"
    expect(feet.altitudeText, "1916.2 ft",
           "a height the vehicle reported in feet is labelled feet, not metres")
    expect(feet.groundSpeedText, "12.5 ft/s", "and a speed carries the units it came with")

    var metric = FlyTelemetry()
    metric.altitude = 584.0
    expect(metric.altitudeText, "584.0 m", "the default stays metric when nothing says otherwise")

    expect(FlyTelemetry().altitudeText, "—", "a height the vehicle has not reported shows nothing")
    expect(FlyTelemetry.measure(1.0, ""), "1.0",
           "a fact with no units at all is printed bare rather than with a guessed suffix")
    expect(FlyTelemetry.measure(-0.04, "m"), "0.0 m",
           "a hair below zero reads as zero, not as minus zero")
}

checkTelemetryUnits()

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
         "altitude": 30.0 as NSNumber, "altitudeUnits": "m",
         "altitudePath": "plan.rallyPointController.points.0.textFieldFacts.2"],
        ["index": 1 as NSNumber, "path": "plan.rallyPointController.points.1",
         "latitude": -35.36 as NSNumber, "longitude": 149.16 as NSNumber,
         "altitude": 164.0 as NSNumber, "altitudeUnits": "ft",
         "altitudePath": "plan.rallyPointController.points.1.textFieldFacts.2"],
    ])
    expect(listed.count == 2, "each rally point the core lists is carried across")
    expect(listed[0].positionText, "-35.362800, 149.166500", "with its position to six places")
    expect(listed[0].altitudeText, "30.0 m",
           "and its height in the operator's units, which the core cooked")
    expect(listed[1].altitudeText, "164 ft",
           "so a station in feet reads in feet rather than being converted twice")
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
              _ hint: String = "") -> [String: Any] {
        ["id": id, "title": title, "invokable": invokable, "complexName": complex,
         "geometry": geometry, "geometryProperty": property, "shapeNoun": noun,
         "placementHint": hint, "simple": (complex is NSNull) as NSNumber]
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
    expect(catalogue.areaProperty(forCommand: "Survey") ?? "", "surveyAreaPolygon",
           "an area pattern reports the polygon it draws from")
    expect(catalogue.lineProperty(forCommand: "Corridor Scan") ?? "", "corridorPolyline",
           "and a line pattern its polyline")
    expect(catalogue.lineProperty(forCommand: "Survey") == nil,
           "an area is not offered as a line, which would append vertices to the wrong property")
    expect(catalogue.areaProperty(forCommand: "Corridor Scan") == nil, "nor the reverse")

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
    expect(reading.stateText, "Disarmed", "a vehicle that is not armed reads as disarmed")

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

    reading.armed = true
    expect(reading.stateText, "Armed", "an armed vehicle on the ground reads as armed")
    reading.flying = true
    expect(reading.stateText, "Flying", "and as flying once it is airborne")

    expect(FlyTelemetry.measure(nil, "m"), "\u{2014}", "a missing altitude shows nothing")
    expect(FlyTelemetry.measure(Double.nan, "m"), "\u{2014}", "and so does a NaN")
    expect(FlyTelemetry.measure(3.26, "m/s"), "3.3 m/s", "speed reads to one decimal")
    expect(FlyTelemetry.measure(-0.0, "m"), "0.0 m", "a vehicle on the ground does not report minus zero")
    expect(FlyTelemetry.measure(-0.04, "m"), "0.0 m", "nor does one a few centimetres below its launch point")
    expect(FlyTelemetry.measure(-0.02, "m/s"), "0.0 m/s", "nor does a stationary one")
    expect(FlyTelemetry.measure(-12.5, "m"), "-12.5 m", "a real negative altitude keeps its sign")
    expect(FlyTelemetry.degrees(91.6), "92\u{00B0}", "heading reads whole degrees")
}

checkFlyTelemetry()

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
    expect(AltitudeMode.choices.count == 4, "a survey offers four altitude modes")
    expect(!AltitudeMode.isChoice(AltitudeMode.mixedRaw), "mixed belongs to a whole mission, not a survey")
    expect(!AltitudeMode.isChoice(5), "none means the distance is not about the ground")
    expect(AltitudeMode.isChoice(AltitudeMode.terrainFrameRaw), "terrain frame is a survey mode")

    expect(AltitudeMode.read(4 as NSNumber) == AltitudeMode.terrainFrameRaw,
           "the mode crosses the bridge as the number of its C++ enum case")
    expect(AltitudeMode.read("AltitudeModeTerrainFrame") == AltitudeMode.none,
           "and a name where a number belongs is no mode at all, which is what broke the pickers")
    expect(AltitudeMode.read(nil) == AltitudeMode.none, "so is a missing one")

    expect(AltitudeMode.title(for: AltitudeMode.terrainFrameRaw), "Follow terrain",
           "the mode reads as what it does, not as its enum name")
    expect(AltitudeMode.title(for: AltitudeMode.relativeRaw), "Relative to launch", "same for relative")
    expect(AltitudeMode.title(for: 9), "Mode 9",
           "an unknown mode is shown as sent rather than hidden")
    expect(AltitudeMode.title(for: AltitudeMode.none), "",
           "but an item that carries no altitude at all names no mode")

    expect(AltitudeMode.usesTerrain(AltitudeMode.terrainFrameRaw), "terrain frame uses the terrain settings")
    expect(AltitudeMode.usesTerrain(AltitudeMode.calcAboveTerrainRaw), "so does calculated above terrain")
    expect(!AltitudeMode.usesTerrain(AltitudeMode.relativeRaw), "relative does not")
    expect(!AltitudeMode.usesTerrain(AltitudeMode.absoluteRaw), "nor does absolute")

    expect(AltitudeMode.missionChoices.count == 5, "a mission offers one more mode than a survey")
    expect(AltitudeMode.isMissionChoice(AltitudeMode.mixedRaw), "a mission can be mixed")
    expect(!AltitudeMode.isChoice(AltitudeMode.mixedRaw), "one survey cannot")
    expect(AltitudeMode.title(for: AltitudeMode.mixedRaw), "Mixed (per item)",
           "mixed says that each item carries its own frame")
}

checkAltitudeMode()

func checkPlanSummary() {
    let metres = Measure.metres
    expect(PlanSummary.distance(0, metres), "\u{2014}", "a plan that goes nowhere shows no distance")
    expect(PlanSummary.distance(-1, metres), "\u{2014}", "nor does a nonsense one")
    expect(PlanSummary.distance(.nan, metres), "\u{2014}", "nor does an unset one")
    expect(PlanSummary.distance(500.397, metres), "500 m", "a distance is whole, as QGC gives it")
    expect(PlanSummary.distance(7047, metres), "7047 m",
           "and stays in the operator's unit rather than rolling into a kilometre I invented")

    let feet = Measure(units: "ft", factor: 3.28084)
    expect(PlanSummary.distance(7047, feet), "23120 ft",
           "on feet the same plan converts instead of printing metres under a foot label")

    expect(PlanSummary.duration(0), "\u{2014}", "no flight time means no duration")
    expect(PlanSummary.duration(.infinity), "\u{2014}", "an infinite estimate is not shown")
    expect(PlanSummary.duration(100.079), "1:40", "minutes and seconds, zero padded")
    expect(PlanSummary.duration(9), "0:09", "under a minute still shows the minute")
    expect(PlanSummary.duration(3661), "1:01:01", "past an hour the hour appears")

    let flight = PlanSummary(distanceMetres: 500.4, seconds: 100.1,
                             maxTelemetryMetres: 457.3, measure: metres)
    expect(flight.hasFlight, "a plan with distance and time has a flight to describe")
    expect(flight.distanceText, "500 m", "the summary formats its own distance")
    expect(flight.durationText, "1:40", "and its own duration")
    expect(flight.telemetryText, "457 m", "and the furthest it gets from launch")

    expect(!PlanSummary.empty.hasFlight, "an empty plan has nothing to summarise")
    expect(!PlanSummary(distanceMetres: 0, seconds: 0, maxTelemetryMetres: 0,
                        measure: metres).hasFlight,
           "a launch point alone is not a flight")
}

checkPlanSummary()

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

    expect(SurveyStats([:]).describes == false,
           "a read that returned nothing describes nothing rather than an empty survey")
}

func checkMeasure() {
    expect(Measure.format(99.4, "m"), "99.4 m", "below a hundred a measure keeps a tenth")
    expect(Measure.format(100, "m"), "100 m", "at a hundred QGC drops to whole numbers")
    expect(Measure.format(89999.17, "m^2"), "89999 m\u{00B2}", "and squares render as a superscript")
    expect(Measure.format(.nan, "m"), "\u{2014}", "a measure that is not a number is not shown as one")

    let feet = Measure(units: "ft", factor: 3.28084)
    expect(feet.text(100), "328 ft", "a metric value crosses into the operator's units before it is shown")
    expect(feet.convert(1) == 3.28084, "the factor is the one the app settings table gives, not one I typed")

    expect(Measure(units: "ft", factor: 0).factor == 1,
           "a factor the bridge could not give falls back to metric rather than collapsing the value")
    expect(Measure(units: "ft", factor: .nan).factor == 1, "so does one that is not a number")
    expect(Measure.metres.text(42), "42.0 m", "and the metric measure is the identity")
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
        "anyConnecting": false as NSNumber, "configuredCount": 2 as NSNumber,
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
    expect(live.configuredCount == 2,
           "configuredCount is the core's own count and is reported separately from the list")

    expect(VideoCamera(["title": "Camera 1"]) == nil,
           "a camera with no slot is dropped, because the slot is what the row is keyed by")
    expect(VideoStatus([:]).cameras.isEmpty, "an empty answer is no cameras")
}
checkVideoStatus()

func checkVideoSources() {
    let live = "[{\"name\":\"\",\"source\":\"RTSP Video Stream\",\"url\":\"\"},"
        + "{\"name\":\"0.0.0.0:5691\",\"source\":\"UDP h.264 Video Stream\",\"url\":\"\"},"
        + "{\"name\":\"\",\"source\":\"Video Stream Disabled\",\"url\":\"\"}]"

    let sources = VideoSources.decode(live)
    expect(sources.count == 3, "every configured slot is read")
    expect(sources[0].title, "Camera 1", "an unnamed slot is named by its number")
    expect(sources[1].title, "0.0.0.0:5691", "a named one keeps its name")
    expect(!sources[2].enabled, "a disabled slot is off")
    expect(sources[2].summary, "Off", "and says so rather than complaining about an address")
    expect(sources[0].misconfigured, "an enabled slot with no address cannot work")
    expect(!sources[2].misconfigured, "a disabled one is not misconfigured, just off")
    expect(sources[1].summary, "No address", "which is what the row reports")

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

func checkDetections() {
    func box(_ label: String, _ x: Double, _ y: Double, _ w: Double, _ h: Double,
             conf: Double? = 0.91) -> [String: Any] {
        var made: [String: Any] = ["label": label, "x": x as NSNumber, "y": y as NSNumber,
                                   "w": w as NSNumber, "h": h as NSNumber]
        if let conf { made["conf"] = conf as NSNumber }
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

func checkPlanMeasuresMatchTheCore() {
    // core-rs read.rs format_measure: a tenth under a hundred, whole numbers at or above it.
    // Measure.format is the head's copy of that rule and its tests pin the same cases; these
    // check that everything in the Plan window actually goes through it, because the terrain
    // sheet and the survey stats on the same screen are the core's own strings.
    expect(Measure.format(40, "m"), "40.0 m", "the shared formatter keeps a tenth under a hundred")
    expect(Measure.format(120, "m"), "120 m", "and drops it at a hundred and above")

    expect(PlanSummary.distance(40, .metres), "40.0 m",
           "a short plan reads the same as a short survey leg, which the core spells 40.0 m")
    expect(PlanSummary.distance(7047, .metres), "7047 m", "and a long one is unchanged")
    expect(PlanSummary.distance(0, .metres), "\u{2014}",
           "a plan with no flight in it still shows a dash rather than 0.0 m")
    expect(PlanSummary.distance(Double.nan, .metres), "\u{2014}", "and so does a number that is not one")

    expect(Measure.reading(120, "m"), "120 m",
           "an item's altitude reads the same as the terrain sheet under it, which the core "
           + "spells 120 m; the head used to write 120.0 m beside it")
    expect(Measure.reading(45.26, "m"), "45.3 m", "and a low one keeps its tenth")
    expect(Measure.reading(0, "m"), "0.0 m",
           "zero is a real altitude, not a missing one, so it is not dashed away")
    expect(Measure.reading(nil, "m"), "\u{2014}", "but an absent one is")
    expect(Measure.format(40, ""), "40.0",
           "a fact with no units reads as a bare number rather than one with a space stranded "
           + "on the end of it")
}

checkPlanMeasuresMatchTheCore()

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

    expect(Preflight.progress(groups, ticked: []), "0 of 4 checked", "an untouched list starts at none")
    expect(!Preflight.ready(groups, ticked: []), "and is not ready")
    let every: Set<String> = ["Hardware", "Battery", "GPS", "Motors"]
    expect(Preflight.ready(groups, ticked: every), "ticking every check is ready")
    expect(Preflight.progress(groups, ticked: every.union(["Payload"])), "4 of 4 checked",
           "a tick left over from another airframe is not counted against a list it is not on")

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
checkOverlayArrange()
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
    let absent = LaunchPosition(home: ["valid": false],
                                item: [:],
                                altitude: ["value": 0 as NSNumber, "units": "m"])
    expect(absent.editable, "no vehicle home means the plan owns the launch position")
    expect(absent.positionText == "Not set", "an unplaced launch position says so")

    let onVehicle = LaunchPosition(home: ["valid": true],
                                   item: ["coordinate": ["latitude": -35.36 as NSNumber,
                                                         "longitude": 149.16 as NSNumber]],
                                   altitude: ["value": 583.0 as NSNumber, "units": "m"])
    expect(!onVehicle.editable, "the vehicle's own home position wins over the plan's")
    expect(onVehicle.altitude == 583.0, "the launch altitude comes from its fact")
    expect(onVehicle.altitudeText, "583 m", "and reads out with the fact's own units")
    expect(onVehicle.positionText == "-35.360000, 149.160000", "a placed launch position reads out")

    let feet = LaunchPosition(home: ["valid": false], item: [:],
                              altitude: ["value": 100.0 as NSNumber, "units": "ft"])
    expect(feet.units == "ft", "the launch altitude takes its units from the fact")
    expect(feet.altitudeText, "100 ft", "so an operator on feet is not told metres")
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

    let running = RemoteSupport(host: "support.ardupilot.org:1234", forwarding: true)
    expect(!running.canConnect, "once forwarding starts there is nothing left to press")
    expect(running.status.contains("restarts"),
           "and the page says it cannot be stopped, because QGC cannot stop it either")

    expect(!RemoteSupport(host: "", forwarding: false).canConnect,
           "a button that would forward to nowhere is refused rather than offered")
    expect(!RemoteSupport.empty.canConnect, "same before the setting has been read")
}

func checkSetupPages() {
    // The names core-rs setup.rs PAGES ships. The head no longer keeps this list -- it decodes
    // view.setup.groups -- so this is an assertion about the glyph table, not a second catalogue.
    // The contract cannot pin it until the recorder captures view.setup.groups, which needs no
    // vehicle and has been asked for.
    let known = ["Summary", "Sensors", "Radio", "Frame", "Flight Modes", "Safety", "Power",
                 "Motors", "Tuning", "Camera", "Lights", "Flight Behavior",
                 "Remote Support", "Parameters"]
    let symbols = known.map(SetupPage.symbol(for:))
    expect(Set(symbols).count == symbols.count,
           "no two pages share an icon, which is how Motors and Remote Support ended up looking alike")
    expect(known.filter { SetupPage.symbol(for: $0) == SetupPage.unknownSymbol }
        .joined(separator: ","), "",
           "every page the core can list has a glyph of its own; Flight Behavior had none until "
           + "the head stopped keeping its own page list and noticed the core had one more")
    expect(SetupPage.symbol(for: "Anything Unknown"), SetupPage.unknownSymbol,
           "and a page added to the core tomorrow draws as unknown rather than as something else")

    let decoded = SetupCatalogue.groups([
        ["title": "Vehicle", "pages": [["name": "Summary", "native": true as NSNumber,
                                        "parameterSections": false as NSNumber]]],
        ["title": "Setup", "pages": [["name": "Safety", "native": true as NSNumber,
                                      "parameterSections": true as NSNumber],
                                     ["name": "Flight Behavior", "native": false as NSNumber,
                                      "parameterSections": false as NSNumber]]],
        ["title": "Empty", "pages": [["name": "Sensors", "native": false as NSNumber,
                                      "parameterSections": false as NSNumber]]],
    ])
    expect(SetupCatalogue.names(decoded).joined(separator: ","),
           "Summary,Safety,Flight Behavior,Sensors",
           "every page the core lists is decoded, native or not")

    let offered = SetupCatalogue.offered(decoded)
    expect(SetupCatalogue.names(offered).joined(separator: ","), "Summary,Safety",
           "a page the core does not claim as native is not offered, because this head has "
           + "nothing to draw for it and the sidebar would open the summary instead")
    expect(offered.map(\.title).joined(separator: ","), "Vehicle,Setup",
           "and a group left with no pages at all goes with them, rather than sitting empty")
    expect(SetupCatalogue.page("Safety", in: offered)?.parameterSections == true,
           "a page carries whether the core has parameter sections for it, which is what lets an "
           + "unknown page still draw instead of falling through to the summary")
    expect(SetupCatalogue.page("Sensors", in: offered) == nil, "and a filtered page is gone")

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
    expect(MapClickAction.refusal(in: MapClickState()).contains("No vehicle"),
           "and says why rather than showing an empty menu")

    var grounded = MapClickState()
    grounded.connected = true
    grounded.roiSupported = true
    grounded.orbitSupported = true
    grounded.homeUsable = true
    expect(MapClickAction.offered(in: grounded) == [.setHome],
           "on the ground only Set home is offered, which is QGC's showSetHome with no flying gate")

    var flying = grounded
    flying.flying = true
    expect(MapClickAction.offered(in: flying) == [.goTo, .orbit, .roi, .setHome, .setHeading],
           "in the air the four guided commands join it, in QGC's own order")

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
    expect(!MapClickAction.offered(in: looking).contains(.roi), "a camera already aimed is not re-aimed")
    expect(MapClickAction.offered(in: looking).contains(.cancelRoi), "it is released instead")

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
           "a fence outside the mission widens Everything without moving Mission")
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

    func shape(_ view: String, _ inner: [String]) -> [String: Any]? {
        var here = shapes[view]
        for step in inner {
            guard let dictionary = here as? [String: Any] else { return nil }
            here = dictionary[step]
            if let list = here as? [Any] { here = list.first }
        }
        return here as? [String: Any]
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
        ("view.battery", [], ["level", "packs"]),
        ("view.battery", ["packs"],
         ["level", "text", "secondaryText", "percent", "voltage", "current"]),
        ("view.preflight", [], ["airframe", "groups"]),
        ("view.preflight", ["groups"], ["name", "checks"]),
        ("view.preflight", ["groups", "checks"],
         ["name", "prompt", "verdict", "reason", "blocked"]),
        ("view.warnings", [], ["showing", "warnings"]),
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
        ("view.setup", ["groups", "pages"], ["name", "native", "parameterSections"]),
        ("view.setup(Safety)", [], ["page", "firmware", "available", "sections"]),
        ("view.setup(Safety)", ["sections"], ["title", "note", "controls"]),
        ("view.setup(Safety)", ["sections", "controls"],
         ["path", "name", "label", "control", "valueString", "display", "units", "options"]),
        ("view.video", [],
         ["available", "gstreamer", "streamSource", "decoding", "streaming", "recording",
          "sourceSize", "activeSource", "multipleSources", "anyConnecting", "configuredCount",
          "summary", "cameras"]),
        ("view.video", ["cameras"],
         ["slot", "title", "status", "connecting", "recording", "configured"]),
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
        ("view.missionKinds", [], ["kinds"]),
        ("view.missionKinds", ["kinds"],
         ["id", "title", "invokable", "complexName", "geometry", "geometryProperty", "shapeNoun",
          "placementHint", "simple"]),
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
        ("view.messages", [], ["count", "items"]),
        ("view.missionSeed(survey,47,8)", ["points"], ["latitude", "longitude"]),
        ("view.links", ["configured"],
         ["index", "path", "name", "type", "typeLabel", "editing", "displaySummary", "connected",
          "autoConnect", "host", "port", "portName", "baud", "filename", "logFileName",
          "lastError"]),
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
    expect(controlKinds.sorted().joined(separator: ","), "choice,number,text,toggle",
           "the four control kinds are the four this editor draws")
    expect(controlKinds.filter { SettingsControl.Kind($0) == .unknown }.joined(separator: ","), "",
           "and every one of them decodes to an editor, rather than falling through to a field")

    expect(recorded("view.setup.firmware").sorted().joined(separator: ","), "apm,none,px4",
           "the three firmware kinds are the three the setup pages are built for")

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
        ("view.links", ["configured"], ["name", "typeLabel", "displaySummary"]),
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
            return expect(false, "\(place) is in the recorded contract")
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
            return expect(false, "\(where_) is in the recorded contract")
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
    expect(polygon?.closed == true, "the core says whether four corners make a polygon")
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

func checkOverlayArrange() {
    let extent = 200.0, size = 1000.0, margin = 10.0, grid = 20.0
    let high = size - margin - extent

    let near = OverlayArrange.snap(100, extent: extent, size: size, margin: margin, grid: grid)
    expect(near == 110, "a drop in the near half snaps to a step measured from the near edge")
    expect((near - margin).truncatingRemainder(dividingBy: grid) == 0,
           "so its distance from that edge is a whole number of steps")

    let far = OverlayArrange.snap(700, extent: extent, size: size, margin: margin, grid: grid)
    expect(far == 690, "a drop in the far half snaps to a step measured from the FAR edge")
    expect((high - far).truncatingRemainder(dividingBy: grid) == 0,
           "so its distance from the far edge is the whole number, which is what keeps a "
           + "right-hand panel looking right-aligned")
    expect(far != 710, "measuring that same drop from the near edge would have given 710")

    expect(OverlayArrange.snap(-500, extent: extent, size: size, margin: margin, grid: grid) == margin,
           "a drop outside the near edge is pulled back to the margin")
    expect(OverlayArrange.snap(5000, extent: extent, size: size, margin: margin, grid: grid) == high,
           "and one outside the far edge to the last position that still fits")
    expect(OverlayArrange.snap(137, extent: extent, size: size, margin: margin, grid: 0) == 137,
           "with no grid the value is kept as dropped")
    expect(OverlayArrange.clamp(5000, extent: extent, size: size, margin: margin) == high,
           "clamping without a grid still keeps the panel on screen")
    expect(OverlayArrange.clamp(400, extent: 5000, size: size, margin: margin) == margin,
           "a panel larger than the window sits at the margin rather than off it")

    expect(OverlayArrange.settles(dropped: 105, base: 100, extent: extent, threshold: 12),
           "a drop close to where the panel started forgets the custom position")
    expect(!OverlayArrange.settles(dropped: 400, base: 100, extent: extent, threshold: 12),
           "a real move keeps it")
    expect(OverlayArrange.settles(dropped: 190, base: 100, extent: extent, threshold: 12),
           "half the panel's own width counts as close, not just the bare threshold")

    expect(OverlayArrange.key(width: 960, height: 640) == "960x640", "sizes key on their pixels")
    let once = OverlayArrange.remember([], "960x640")
    expect(once == ["960x640"], "the first arrangement is remembered")
    expect(OverlayArrange.remember(["800x600", "960x640"], "800x600") == ["960x640", "800x600"],
           "re-arranging a size moves it to the end rather than duplicating it")
    let many = (1...10).reduce([String]()) { OverlayArrange.remember($0, "\($1)00x600") }
    expect(many.count == OverlayArrange.maxRememberedSizes, "only eight sizes are kept")
    expect(many.first == "300x600" && many.last == "1000x600",
           "and it is the oldest two that fall off the front")

    expect(OverlayArrange.storedKey(["800x600", "960x640"], width: 960, height: 640) == "960x640",
           "an exact size wins")
    expect(OverlayArrange.storedKey(["800x600", "1400x900"], width: 960, height: 640) == "800x600",
           "otherwise the nearest remembered size is a better start than nothing")
    expect(OverlayArrange.storedKey([], width: 960, height: 640) == nil,
           "with nothing remembered the panel stays where it was designed to be")
    expect(OverlayArrange.distance(from: "junk", width: 960, height: 640) == nil,
           "a malformed key has no distance")
    expect(OverlayArrange.storedKey(["junk", "800x600"], width: 960, height: 640) == "800x600",
           "and is never chosen as the nearest")
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
    expect(MessageRateChoice.shown(17, in: choices) == 0,
           "and a rate the picker cannot show falls back to Default rather than selecting nothing")
    expect(MessageRateChoice.shown(-1, in: choices) == -1, "Off shows as Off")

    expect(MessageRateChoice(["title": "5 Hz"]) == nil, "a choice with no rate is dropped")
}
