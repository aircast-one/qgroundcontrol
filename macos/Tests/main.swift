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

// humanise turns C++ identifiers into sidebar and row labels. It was wrong twice:
// splitting every capital ("Remote I D") and losing lowercase-leading acronyms ("Rtk").
expect(Fact.humanise("remoteIDSettings"), "Remote ID Settings", "acronym run kept together")
expect(Fact.humanise("adsbVehicleManager"), "ADSB Vehicle Manager", "leading lowercase acronym")
expect(Fact.humanise("rtkSettings"), "RTK Settings", "short leading acronym")
expect(Fact.humanise("viewer3D"), "Viewer 3D", "letter to digit boundary")
expect(Fact.humanise("apmMavlinkStreamRate"), "APM Mavlink Stream Rate", "acronym then words")
expect(Fact.humanise("offlineEditingCruiseSpeed"), "Offline Editing Cruise Speed", "plain camel case")
expect(Fact.humanise(""), "", "empty identifier")
expect(Fact.humanise("x"), "X", "single character")

// Every storage group must appear on exactly one page: a missing group is a setting the
// operator can no longer reach, a duplicated one is a setting with two homes.
let groups = SettingsPage.all.flatMap { $0.sections.map(\.group) }
expect(Set(groups).count == groups.count, "no group appears on two pages")

// Fact decoding: the bridge's JSON shape drives every control choice.
let boolFact = Fact(json: ["name": "muted", "typeIsBool": true, "value": true], groupPath: "settings.app")
expect(boolFact != nil, "bool fact parses")
if case .toggle = boolFact!.kind {} else { expect(false, "bool fact yields a toggle") }
expect(boolFact!.path, "settings.app.muted", "path is group-qualified")

let enumFact = Fact(json: ["name": "speedUnits", "enumStrings": ["Feet", "Meters"],
                           "enumValues": [0, 1], "value": 1], groupPath: "settings.units")
if case let .choice(labels, values) = enumFact!.kind {
    expect(labels.count == 2 && values == [0, 1], "enum labels and values pair up")
} else { expect(false, "enum fact yields a choice") }

// enumValues is sometimes absent even when enumStrings is not; fall back to positions.
let looseEnum = Fact(json: ["name": "x", "enumStrings": ["A", "B", "C"], "value": 2], groupPath: "g")
if case let .choice(_, values) = looseEnum!.kind {
    expect(values == [0, 1, 2], "missing enumValues falls back to indices")
} else { expect(false, "loose enum yields a choice") }

// QGC leaves min/max at the type extremes when unbounded; those must not become hints.
let unbounded = Fact(json: ["name": "n", "min": 0, "max": 4294967295, "value": 3], groupPath: "g")
if case let .number(_, minimum, maximum) = unbounded!.kind {
    expect(minimum == 0 && maximum == nil, "type-extreme maximum is dropped")
} else { expect(false, "numeric fact yields a number") }

expect(Fact(json: ["value": 1], groupPath: "g") == nil, "a fact without a name is rejected")

// A live LinkInterface is reported by the bridge as a child, not a value; reading
// json["link"] instead reports every connected link as disconnected.
let liveLink = LinkConfig(index: 0, json: ["name": "sitl", "linkType": "TypeTcp",
                                           "children": ["link"], "link": NSNull()])
expect(liveLink.connected, "a link listed in children reads as connected")
let deadLink = LinkConfig(index: 1, json: ["name": "sitl", "linkType": "TypeTcp",
                                           "children": [], "link": NSNull()])
expect(!deadLink.connected, "a link absent from children reads as disconnected")
expect(liveLink.typeLabel, "TCP", "TypeTcp renders as TCP")
expect(LinkConfig(index: 2, json: ["linkType": "TypeLogReplay"]).typeLabel, "Log Replay", "TypeLogReplay renders readably")

// A TCP link with no host cannot connect; "TCP · :5760" hid that.
expect(LinkConfig(index: 0, json: ["linkType": "TypeTcp", "host": "", "summary": ":5760"]).displaySummary,
       "No host set", "hostless TCP link says so")
expect(LinkConfig(index: 0, json: ["linkType": "TypeTcp", "host": "h", "summary": "h:5760"]).displaySummary,
       "h:5760", "TCP link with a host keeps its summary")
expect(LinkConfig(index: 0, json: ["linkType": "TypeUdp", "summary": "UDP port 14550"]).displaySummary,
       "UDP port 14550", "UDP has no host and is not flagged")

// UDP exposes localPort, TCP exposes port; reading only "port" reported UDP links as port 0.
expect(String(LinkConfig(index: 0, json: ["linkType": "TypeUdp", "localPort": 14550]).port),
       "14550", "UDP port comes from localPort")
expect(String(LinkConfig(index: 0, json: ["linkType": "TypeTcp", "port": 5760]).port),
       "5760", "TCP port comes from port")

func expectEditing(_ type: String, _ want: LinkConfig.Editing, _ label: String) {
    expect(LinkConfig(index: 0, json: ["linkType": type]).editing == want, label)
}
expectEditing("TypeTcp", .hostAndPort, "TCP edits host and port")
expectEditing("TypeUdp", .portOnly, "UDP edits only its local port")
expectEditing("TypeSerial", .serial, "serial edits device and baud")
expectEditing("TypeMock", LinkConfig.Editing.none, "mock link has nothing to edit")

// The thresholds are ArduPilot/PX4 flight guidance, not styling: getting them wrong
// tells an operator a shaking airframe is fine.
func expectSeverity(_ value: Double, _ want: VibrationReading.Severity, _ label: String) {
    expect(VibrationReading.severity(value) == want, label)
}
expectSeverity(0, .normal, "zero vibration is normal")
expectSeverity(29.9, .normal, "just under the warning threshold is normal")
expectSeverity(30, .warning, "the warning threshold is inclusive")
expectSeverity(59.9, .warning, "just under the danger threshold is a warning")
expectSeverity(60, .danger, "the danger threshold is inclusive")
expectSeverity(1000, .danger, "beyond the scale is still danger")

// worst() drives the advice line, so it must track the highest axis, not the last one.
expect(VibrationReading(x: 5, y: 65, z: 5, clipCounts: [], available: true).worst == .danger,
       "one bad axis makes the whole reading dangerous")
expect(VibrationReading(x: 5, y: 5, z: 35, clipCounts: [], available: true).worst == .warning,
       "the worst axis wins")
expect(!VibrationReading.unavailable.available, "the unavailable reading reports itself as such")

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

// A disabled sensor also reports unhealthy. Treating that as a fault would put
// Geofence and Logging beside a failed GPS and cry wolf before every flight.
expect(SensorHealth(name: "Geofence", enabled: false, healthy: false).state == .disabled,
       "a disabled sensor is not a fault")
expect(SensorHealth(name: "GPS", enabled: true, healthy: false).state == .unhealthy,
       "an enabled sensor that is unhealthy is a fault")
expect(SensorHealth(name: "Gyro", enabled: true, healthy: true).state == .healthy,
       "an enabled healthy sensor is healthy")

let parsed = SensorHealth.from(json: [
    "sensorNames": ["GPS", "Gyro", "Logging"],
    "sensorEnabled": [true, true, false],
    "sensorHealthy": [false, true, false]])
expect(String(parsed.count), "3", "all three sensors parse")

// Faults first: the operator is looking for what is wrong.
let ordered = SensorHealth.ordered(parsed)
expect(ordered.map(\.name).joined(separator: ","), "GPS,Gyro,Logging", "faults sort ahead of healthy, disabled last")

// Mismatched array lengths mean a malformed payload; inventing sensors would be worse.
expect(SensorHealth.from(json: ["sensorNames": ["GPS", "Gyro"],
                                "sensorEnabled": [true],
                                "sensorHealthy": [true]]).isEmpty,
       "a ragged payload yields nothing rather than guessing")

// A section listing parameters the firmware does not have would imply settings the
// operator cannot change; an empty section is dropped entirely.
let copterLike: Set<String> = ["FS_THR_ENABLE", "FS_THR_VALUE", "RTL_ALT", "ARMING_CHECK"]
let present = SafetySection.present(in: copterLike)
expect(present.map(\.section.title).joined(separator: ","),
       "Failsafe,Return to Launch,Arming", "only sections with present parameters survive")
expect(present.first!.names.joined(separator: ","), "FS_THR_ENABLE,FS_THR_VALUE",
       "a section keeps only the parameters this vehicle has")
expect(SafetySection.present(in: []).isEmpty, "a vehicle with none of them gets no sections")
expect(SafetySection.present(in: ["RTL_ALT"]).count == 1, "one parameter is enough to keep its section")

// Order is the authored order, not whatever the vehicle happens to report.
expect(SafetySection.all.map(\.title).first!, "Failsafe", "failsafe leads the page")

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
    "facts": [["name": "Altitude", "value": 50.0]],
    "specifiesAltitude": true], index: 1)
expect(waypoint.altitudeText, "50.0 m", "altitude comes from the fact")
expect(waypoint.positionText, "-35.362900, 149.165000", "position formats to six decimals")
expect(waypoint.hasPosition, "a waypoint with a coordinate has a position")

let start = MissionItem(json: [
    "sequenceNumber": 0, "commandName": "Mission Start", "isCurrentItem": true,
    "coordinate": ["latitude": -35.36, "longitude": 149.16, "altitude": 584.09],
    "facts": []], index: 0)
expect(start.altitudeText, "584.1 m", "falls back to the coordinate altitude")
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
    let polygon = FenceShape(json: [
        "inclusion": true, "count": 4, "area": 85268.0,
        "center": ["latitude": -35.3635, "longitude": 149.1655],
        "path": [["latitude": -35.3607, "longitude": 149.1612],
                 ["latitude": -35.3607, "longitude": 149.1687],
                 ["latitude": -35.3652, "longitude": 149.1687],
                 ["latitude": -35.3652, "longitude": 149.1612]],
    ], id: 0, circle: false)
    expect(polygon.vertices.count == 4, "a polygon keeps every vertex the bridge sent")
    expect(polygon.radius == nil, "a polygon has no radius")
    expect(polygon.framingPoints.count == 4, "a polygon frames from its vertices")
    expect(polygon.kindText, "Keep-in polygon", "an inclusion polygon reads as keep-in")

    let exclusion = FenceShape(json: ["inclusion": false, "count": 3, "path": []], id: 1, circle: false)
    expect(exclusion.kindText, "Keep-out polygon", "an exclusion polygon reads as keep-out")

    let circle = FenceShape(json: [
        "inclusion": true,
        "center": ["latitude": -35.3635, "longitude": 149.1655],
        "facts": [["name": "Radius", "value": 100.0]],
    ], id: 2, circle: true)
    expect(circle.radius != nil, "a circle carries its radius")
    expect(circle.vertices.isEmpty, "a circle has no vertices")
    expect(circle.framingPoints.count == 2, "a circle frames from a box around its radius")
    let span = circle.framingPoints[1].latitude - circle.framingPoints[0].latitude
    expect(abs(span - 200.0 / 111_320.0) < 1e-6, "the framing box spans the circle's diameter")

    let headless = FenceShape(json: ["inclusion": true, "facts": []], id: 3, circle: true)
    expect(headless.framingPoints.isEmpty, "a circle with no centre contributes no framing points")

    let bogus = FenceShape(json: [
        "count": 2, "path": [["latitude": "north", "longitude": 149.16],
                             ["latitude": -35.36, "longitude": 149.17]],
    ], id: 4, circle: false)
    expect(bogus.vertices.count == 1, "a malformed vertex is dropped rather than read as zero")
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
    let complete = VehicleComponentInfo(json: ["name": "Radio", "setupComplete": true, "requiresSetup": true])!
    let missing = VehicleComponentInfo(json: ["name": "Sensors", "setupComplete": false, "requiresSetup": true])!
    let optional = VehicleComponentInfo(json: ["name": "Camera", "setupComplete": false, "requiresSetup": false])!

    expect(!complete.needsAttention, "a complete component needs no attention")
    expect(missing.needsAttention, "an incomplete required component needs attention")
    expect(!optional.needsAttention, "an incomplete optional component is not a blocker")
    expect(VehicleComponentInfo(json: ["setupComplete": true]) == nil, "a nameless component is dropped")

    let healthy = VehicleReadiness(connected: true, components: [complete, optional], sensorFaults: [])
    expect(healthy.ready, "setup complete with healthy sensors is ready")
    expect(healthy.headline, "Ready to fly", "and says so")

    let faulty = VehicleReadiness(connected: true, components: [complete, optional],
                                  sensorFaults: ["Gyro", "Accelerometer"])
    expect(!faulty.ready, "a sensor fault is never ready, however complete the setup")
    expect(faulty.headline, "2 sensors reporting a fault", "the headline names the fault, not readiness")
    expect(faulty.detail, "Gyro, Accelerometer", "and says which sensors")

    let unset = VehicleReadiness(connected: true, components: [complete, missing], sensorFaults: [])
    expect(!unset.ready, "an incomplete required component is not ready")
    expect(unset.headline, "1 component needs setup", "the headline counts outstanding components")

    let offline = VehicleReadiness(connected: false, components: [], sensorFaults: [])
    expect(!offline.ready, "no vehicle is never ready")
    expect(offline.headline, "No vehicle connected", "and says why")

    let bare = VehicleReadiness(connected: true, components: [], sensorFaults: [])
    expect(!bare.ready, "a vehicle reporting no components is not declared ready")
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
    let entry = LogEntry(json: [
        "id": 3, "size": 898330, "status": "Available", "received": true,
        "time": "2026-09-07T16:29:00.000",
    ])
    expect(entry != nil, "a log entry parses from what the controller reports")
    expect(entry?.sizeText == "877.3 KB", "size reads in KB with a decimal point, not a comma")
    expect(entry?.id == 3, "the id is kept for downloading")
    expect(entry?.time.contains("2026") == true, "the timestamp is rendered, not passed through raw")

    expect(LogEntry.humanSize(0), "0 bytes", "an empty log reads as bytes")
    expect(LogEntry.humanSize(512), "512 bytes", "under a kilobyte stays in bytes")
    expect(LogEntry.humanSize(1024 * 1024 * 3), "3.0 MB", "megabytes read as megabytes")
    expect(LogEntry.humanTime("not a date"), "not a date", "an unparseable time is shown as sent")
    expect(LogEntry(json: ["size": 10]) == nil, "an entry without an id is dropped")
}

checkLogEntry()

func checkTerrainProfile() {
    let profile = TerrainProfile(points: [
        TerrainPoint(distance: 0, missionAltitude: 585, terrainAltitude: 585, collision: false),
        TerrainPoint(distance: 300, missionAltitude: 627, terrainAltitude: 590, collision: false),
        TerrainPoint(distance: 570, missionAltitude: 627, terrainAltitude: 640, collision: true),
    ])
    expect(profile.usable, "three placed points make a profile")
    expect(profile.hasCollision, "a point below terrain is a collision")
    expect(profile.unknownTerrain == 0, "every point here has terrain")
    expect(profile.totalDistance == 570, "the profile spans the furthest point")
    expect(profile.minAltitude < 585, "the band leaves room below the lowest altitude")
    expect(profile.maxAltitude > 640, "and above the highest")

    expect(profile.x(profile.points[2], width: 100) == 100, "the last point sits at the right edge")
    expect(profile.x(profile.points[0], width: 100) == 0, "the first sits at the left")
    let top = profile.y(profile.maxAltitude, height: 50)
    let bottom = profile.y(profile.minAltitude, height: 50)
    expect(abs(top) < 0.001, "the highest altitude maps to the top of the plot")
    expect(abs(bottom - 50) < 0.001, "the lowest maps to the bottom")

    let flat = TerrainProfile(points: [
        TerrainPoint(distance: 0, missionAltitude: 100, terrainAltitude: 100, collision: false),
        TerrainPoint(distance: 10, missionAltitude: 100, terrainAltitude: 100, collision: false),
    ])
    expect(flat.usable, "a flat mission over flat ground still plots")
    expect(flat.maxAltitude > flat.minAltitude, "and is given a band rather than zero range")

    let unknown = TerrainProfile(points: [
        TerrainPoint(distance: 0, missionAltitude: 100, terrainAltitude: nil, collision: false),
        TerrainPoint(distance: 5, missionAltitude: 100, terrainAltitude: 90, collision: false),
    ])
    expect(unknown.unknownTerrain == 1, "a point with no terrain data is counted")
    expect(!unknown.groundKnown, "a partly known ground is not drawn as if it were the ground")
    expect(profile.groundKnown, "a fully known ground is drawn")

    expect(!TerrainProfile.empty.usable, "an empty profile is not drawn")
    expect(TerrainProfile(points: []).totalDistance == 0, "no points span no distance")
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
    ], list: "textFieldFacts")

    expect(facts.count == 2, "a fact with no name is dropped")
    expect(facts[0].id, "textFieldFacts.0", "a fact is addressed by its list and position")
    expect(facts[1].id, "textFieldFacts.1", "positions follow the order the controller reported")
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

func checkMissionItemKinds() {
    expect(MissionItemKind.allCases.count == 4, "the add menu offers four kinds of item")
    expect(MissionItemKind.waypoint.invokable, "insertSimpleMissionItem", "a waypoint inserts a simple item")
    expect(MissionItemKind.takeoff.invokable, "insertTakeoffItem", "takeoff has its own insert")
    expect(MissionItemKind.land.invokable, "insertLandItem", "land has its own insert")
    expect(MissionItemKind.roi.invokable, "insertROIMissionItem", "a region of interest has its own insert")
    expect(MissionItemKind(rawValue: "takeoff") == .takeoff, "a kind round-trips through its raw value")
    expect(MissionItemKind(rawValue: "survey") == nil, "an unknown kind is not invented")
    expect(MissionItemKind.roi.placementHint, "Click the map to place a region of interest.",
           "the hint reads as English rather than a lowercased title")
    expect(MissionItemKind.takeoff.placementHint, "Click the map to place a takeoff.",
           "and names the kind being placed")
}

checkMissionItemKinds()

if failures == 0 {
    print("all Swift checks passed")
    exit(0)
}
FileHandle.standardError.write("\(failures) check(s) failed\n".data(using: .utf8)!)
exit(1)
