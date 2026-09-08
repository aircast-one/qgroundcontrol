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
let present = SetupSection.present(SetupSection.safety, in: copterLike)
expect(present.map(\.section.title).joined(separator: ","),
       "Failsafe,Return to Launch,Arming", "only sections with present parameters survive")
expect(present.first!.names.joined(separator: ","), "FS_THR_ENABLE,FS_THR_VALUE",
       "a section keeps only the parameters this vehicle has")
expect(SetupSection.present(SetupSection.safety, in: []).isEmpty, "a vehicle with none of them gets no sections")
expect(SetupSection.present(SetupSection.safety, in: ["RTL_ALT"]).count == 1, "one parameter is enough to keep its section")

// Order is the authored order, not whatever the vehicle happens to report.
expect(SetupSection.safety.map(\.title).first!, "Failsafe", "failsafe leads the page")

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
    expect(polygon.rowDetail, polygon.detailText,
           "a polygon's row carries its vertex count and area")
    expect(FenceShape(json: ["center": ["latitude": -35.36, "longitude": 149.16],
                             "facts": [["name": "Radius", "value": 250]]],
                      id: 1, circle: true).rowDetail, "",
           "a circle's row does not repeat the radius the field beside it already shows")

    expect(polygon.shapeText, "Polygon",
           "the row names only the shape; the seal and the picker beside it carry keep-in or keep-out")

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
    ], list: "textFieldFacts")

    expect(facts.count == 2, "a fact with no name is dropped")
    expect(facts[0].id, "textFieldFacts.0", "a fact is addressed by its list and position")
    expect(facts[1].id, "textFieldFacts.1", "positions follow the order the controller reported")

    let owned = ItemFact.owned([
        ["name": "Grid angle", "property": "gridAngle", "valueString": "0", "units": "deg"],
        ["name": "Refly", "property": "refly90Degrees", "valueString": "false", "readOnly": true],
        ["name": "No property", "valueString": "1"],
    ])
    expect(owned.count == 2, "a fact with no property is not addressable and is dropped")
    expect(owned[0].id, "gridAngle", "an item's own fact is addressed by its property")
    expect(!owned[0].readOnly, "an editable fact is editable")
    expect(owned[1].readOnly, "a read-only fact says so")
    expect(owned[0].title, "Grid angle", "a described fact shows its description")

    let identifiers = ItemFact.owned([
        ["name": "TurnAroundDistanceMultiRotor", "property": "turnAroundDistance", "valueString": "10"],
        ["name": "HoverAndCapture", "property": "hoverAndCapture", "valueString": "false", "typeIsBool": true],
    ])
    let cameraFacts = ItemFact.camera([
        ["name": "SensorWidth", "property": "sensorWidth", "valueString": "7.6", "units": "mm"],
        ["name": "FrontalOverlap", "property": "frontalOverlap", "valueString": "70", "units": "%"],
        ["name": "DistanceToSurface", "property": "distanceToSurface", "valueString": "50", "units": "m"],
        ["name": "SideOverlap", "property": "sideOverlap", "valueString": "70", "units": "%"],
        ["name": "ImageDensity", "property": "imageDensity", "valueString": "1.8", "units": "cm/px"],
    ])
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

func checkGuidedValue() {
    expect(GuidedValue(label: "x", units: "m", minimum: 10, maximum: 10, initial: 10) == nil,
           "a range with no span is no range; a slider over it cannot be moved")
    expect(GuidedValue(label: "x", units: "m", minimum: 5, maximum: 120, initial: .nan) == nil,
           "and a vehicle that has not reported the value gives none either")

    guard let takeoff = GuidedValue.takeoff(minimumAltitude: 3, maximumAltitude: 121) else {
        expect(false, "a takeoff range is built from the firmware minimum and the setting maximum")
        return
    }
    expect(takeoff.initial == 3, "takeoff starts at the lowest the firmware allows, as QGC does")
    expect(takeoff.clamped(500) == 121, "and nothing above the setting maximum can be chosen")
    expect(takeoff.clamped(0) == 3, "nor below the firmware minimum")
    expect(takeoff.text(50), "50 m", "a height reads in whole metres")

    guard let above = GuidedValue.altitude(minimum: 5, maximum: 121, current: 40) else {
        expect(false, "a change-altitude range is built from the settings and the current height")
        return
    }
    expect(above.initial == 40, "it opens at the height the vehicle is already at")

    let clipped = GuidedValue.altitude(minimum: 5, maximum: 121, current: 400)
    expect(clipped?.initial == 121,
           "a vehicle already above the ceiling opens at the ceiling, not off the end of the slider")

    let ground = GuidedValue.speed(maximum: 12, forwardFlight: false,
                                   minimumAirspeed: 15, maximumAirspeed: 30)
    expect(ground?.label ?? "", "Ground speed", "a multirotor changes ground speed")
    expect(ground?.initial == 6, "opening at half the limit, as QGC does")
    expect(ground?.text(6.25) ?? "", "6.2 m/s", "and a speed reads to a tenth")

    let air = GuidedValue.speed(maximum: 12, forwardFlight: true,
                                minimumAirspeed: 15, maximumAirspeed: 30)
    expect(air?.label ?? "", "Airspeed", "a vehicle in forward flight changes airspeed instead")
    expect(air?.initial == 22.5, "opening midway between the firmware's own limits")
}

checkGuidedValue()

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

func checkFenceUsable() {
    let square = (0..<4).map { _ in ["latitude": -35.36, "longitude": 149.16] }
    let polygon = FenceShape(json: ["count": 4, "path": square,
                                    "center": ["latitude": -35.36, "longitude": 149.16]],
                             id: 0, circle: false)
    expect(polygon.usable, "a polygon with its vertices is usable")

    let hollow = FenceShape(json: ["count": 4, "path": [NSNull(), NSNull(), NSNull(), NSNull()]],
                            id: 0, circle: false)
    expect(!hollow.usable,
           "a polygon that counts four vertices but carries none is not; it draws nothing")

    let circle = FenceShape(json: ["center": ["latitude": -35.36, "longitude": 149.16],
                                   "facts": [["name": "Radius", "value": 200]]],
                            id: 1, circle: true)
    expect(circle.usable, "a circle needs a centre, not vertices")
    expect(!FenceShape(json: [:], id: 1, circle: true).usable, "and without one it is not usable")
}

checkFenceUsable()

func checkRallyAndBreach() {
    let placed = RallyPointRow(json: ["coordinate": ["latitude": -35.3628, "longitude": 149.1665,
                                                     "altitude": 60.0]], id: 0)
    expect(placed.altitudeText, "60.0 m", "a rally point shows the height it holds")
    expect(placed.positionText, "-35.362800, 149.166500", "and where it is")

    let bare = RallyPointRow(json: [:], id: 1)
    expect(bare.altitudeText, "—", "a point with no coordinate claims no height")
    expect(bare.positionText, "—", "nor a position")

    let nan = RallyPointRow(json: ["coordinate": ["latitude": -35.36, "longitude": 149.16,
                                                  "altitude": Double.nan]], id: 2)
    expect(nan.altitudeText, "—",
           "and a height the vehicle reported as not-a-number is not shown as one")
}

checkRallyAndBreach()

func checkMissionItemKinds() {
    expect(MissionItemKind.allCases.count == 7,
           "the add menu offers every item type, including the three survey patterns")
    expect(MissionItemKind.survey.complexName == "Survey", "a survey inserts by name as a complex item")
    expect(MissionItemKind.waypoint.complexName == nil, "a waypoint is not a complex item")
    expect(MissionItemKind.survey.placementHint, "Click the map to place a survey area.",
           "the hint says area rather than survey")

    let area = MissionItemKind.defaultArea(latitude: -35.363, longitude: 149.165)
    expect(area.count == 4, "a new survey gets a four cornered area rather than an empty one")
    expect(area[0].latitude < -35.363 && area[2].latitude > -35.363, "the area straddles the point")
    expect(area[0].longitude < 149.165 && area[2].longitude > 149.165, "on both axes")
    let span = (area[2].latitude - area[0].latitude) * 111_320
    expect(abs(span - 2 * MissionItemKind.defaultAreaMetres) < 1, "and is the intended size across")
    expect(MissionItemKind(rawValue: "") == nil,
           "an empty raw value is no kind, which is how the Empty template asks for nothing")
    expect(MissionItemKind(rawValue: "survey") == .survey,
           "and a kind survives the round trip through its raw value")

    expect(MissionItemKind.shapeImportable.map(\.rawValue).joined(separator: ","),
           "survey,corridor,structure",
           "only the three complex patterns can be drawn from a shape file")
    expect(MissionItemKind.shapeImportable.allSatisfy { $0.complexName != nil },
           "and every one of them has a name the controller inserts by")
    expect(MissionItemKind.corridor.shapeNoun, "path", "a corridor is imported from a path")
    expect(MissionItemKind.survey.shapeNoun, "area", "a survey from an area")
    expect(MissionItemKind.structure.shapeNoun, "area", "a structure scan from an area too")

    expect(MissionItemKind.waypoint.invokable, "insertSimpleMissionItem", "a waypoint inserts a simple item")
    expect(MissionItemKind.takeoff.invokable, "insertTakeoffItem", "takeoff has its own insert")
    expect(MissionItemKind.land.invokable, "insertLandItem", "land has its own insert")
    expect(MissionItemKind.roi.invokable, "insertROIMissionItem", "a region of interest has its own insert")
    expect(MissionItemKind(rawValue: "takeoff") == .takeoff, "a kind round-trips through its raw value")
    expect(MissionItemKind(rawValue: "helix") == nil, "an unknown kind is not invented")

    expect(MissionItemKind.corridor.complexName ?? "", "Corridor Scan",
           "the pattern names match what the vehicle offers in complexMissionItemNames")
    expect(MissionItemKind.structure.complexName ?? "", "Structure Scan", "for all three")
    expect(MissionItemKind.corridor.invokable, "insertComplexMissionItem",
           "every pattern goes in through the complex insert")

    expect(MissionItemKind.survey.geometry == .area("surveyAreaPolygon"),
           "a survey is seeded with an area")
    expect(MissionItemKind.structure.geometry == .area("structurePolygon"),
           "so is a structure scan, but into its own polygon")
    expect(MissionItemKind.corridor.geometry == .line("corridorPolyline"),
           "a corridor is a line, not an area, so a four-corner box would be wrong")
    expect(MissionItemKind.waypoint.geometry == .none, "a simple item has no geometry to seed")

    let seededArea = MissionItemKind.seed(for: .survey, latitude: -35.36, longitude: 149.16)
    expect(seededArea?.points.count == 4, "an area is seeded as a box")
    expect(seededArea?.property ?? "", "surveyAreaPolygon", "on the property that item uses")
    let seededLine = MissionItemKind.seed(for: .corridor, latitude: -35.36, longitude: 149.16)
    expect(seededLine?.points.count == 2, "a corridor is seeded as two ends")
    expect(seededLine?.property ?? "", "corridorPolyline", "on its polyline")
    expect(MissionItemKind.seed(for: .waypoint, latitude: 0, longitude: 0) == nil,
           "and a waypoint is seeded with nothing at all")

    expect(MissionItemKind.areaProperty(forCommand: "Survey") ?? "", "surveyAreaPolygon",
           "the map finds a survey's outline by the item's own command name")
    expect(MissionItemKind.areaProperty(forCommand: "Structure Scan") ?? "", "structurePolygon",
           "and a structure scan's, which is a different property entirely")
    expect(MissionItemKind.areaProperty(forCommand: "Corridor Scan") == nil,
           "a corridor is a line and has no area to draw, so it is not offered one")
    expect(MissionItemKind.areaProperty(forCommand: "Waypoint") == nil,
           "and a simple item has none either")

    expect(MissionItemKind.lineProperty(forCommand: "Corridor Scan") ?? "", "corridorPolyline",
           "a corridor's path is found the same way, by the item's command name")
    expect(MissionItemKind.lineProperty(forCommand: "Survey") == nil,
           "a survey is an area, so it is never asked for a line")
    expect(MissionItemKind.lineProperty(forCommand: "Structure Scan") == nil,
           "nor is a structure scan")
    expect(MissionItemKind.lineProperty(forCommand: "Waypoint") == nil,
           "and a simple item has no path of its own")
    expect(MissionItemKind.roi.placementHint, "Click the map to place a region of interest.",
           "the hint reads as English rather than a lowercased title")
    expect(MissionItemKind.takeoff.placementHint, "Click the map to place a takeoff.",
           "and names the kind being placed")
}

checkMissionItemKinds()

func checkFlyTelemetry() {
    var reading = FlyTelemetry()
    expect(reading.batteryLevel == .unknown, "no battery reading is unknown, not good")
    expect(reading.gpsLevel == .unknown, "no gps reading is unknown, not good")
    expect(reading.batteryText, "\u{2014}", "and shows nothing rather than a number")
    expect(reading.stateText, "Disarmed", "a vehicle that is not armed reads as disarmed")

    reading.batteryPercent = 100
    reading.batteryVolts = 12.6
    expect(reading.batteryLevel == .good, "a full battery is good")
    expect(reading.batteryText, "100% \u{00B7} 12.6 V", "and reads as percent and volts")

    reading.batteryPercent = 20
    expect(reading.batteryLevel == .warning, "a fifth of a battery is a warning")
    reading.batteryPercent = 10
    expect(reading.batteryLevel == .critical, "a tenth of a battery is critical")

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

    expect(FlyTelemetry.metres(nil), "\u{2014}", "a missing altitude shows nothing")
    expect(FlyTelemetry.metres(Double.nan), "\u{2014}", "and so does a NaN")
    expect(FlyTelemetry.speed(3.26), "3.3 m/s", "speed reads to one decimal")
    expect(FlyTelemetry.metres(-0.0), "0.0 m", "a vehicle on the ground does not report minus zero")
    expect(FlyTelemetry.metres(-0.04), "0.0 m", "nor does one a few centimetres below its launch point")
    expect(FlyTelemetry.speed(-0.02), "0.0 m/s", "nor does a stationary one")
    expect(FlyTelemetry.metres(-12.5), "-12.5 m", "a real negative altitude keeps its sign")
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
    expect(!AltitudeMode.isChoice("AltitudeModeMixed"), "mixed belongs to a whole mission, not a survey")
    expect(!AltitudeMode.isChoice("AltitudeModeNone"), "none means the distance is not about the ground")
    expect(AltitudeMode.isChoice("AltitudeModeTerrainFrame"), "terrain frame is a survey mode")

    expect(AltitudeMode.title(for: "AltitudeModeTerrainFrame"), "Follow terrain",
           "the mode reads as what it does, not as its enum name")
    expect(AltitudeMode.title(for: "AltitudeModeRelative"), "Relative to launch", "same for relative")
    expect(AltitudeMode.title(for: "SomethingNew"), "SomethingNew",
           "an unknown mode is shown as sent rather than hidden")

    expect(AltitudeMode.usesTerrain("AltitudeModeTerrainFrame"), "terrain frame uses the terrain settings")
    expect(AltitudeMode.usesTerrain("AltitudeModeCalcAboveTerrain"), "so does calculated above terrain")
    expect(!AltitudeMode.usesTerrain("AltitudeModeRelative"), "relative does not")
    expect(!AltitudeMode.usesTerrain("AltitudeModeAbsolute"), "nor does absolute")

    expect(AltitudeMode.missionChoices.count == 5, "a mission offers one more mode than a survey")
    expect(AltitudeMode.isMissionChoice("AltitudeModeMixed"), "a mission can be mixed")
    expect(!AltitudeMode.isChoice("AltitudeModeMixed"), "one survey cannot")
    expect(AltitudeMode.title(for: "AltitudeModeMixed"), "Mixed (per item)",
           "mixed says that each item carries its own frame")
}

checkAltitudeMode()

func checkPlanSummary() {
    expect(PlanSummary.distance(0), "—", "a plan that goes nowhere shows no distance")
    expect(PlanSummary.distance(-1), "—", "nor does a nonsense one")
    expect(PlanSummary.distance(.nan), "—", "nor does an unset one")
    expect(PlanSummary.distance(500.397), "500 m", "metres below a kilometre, rounded")
    expect(PlanSummary.distance(999.6), "1000 m", "just under the switch is still metres")
    expect(PlanSummary.distance(1000), "1.0 km", "a kilometre reads as kilometres")
    expect(PlanSummary.distance(12345), "12.3 km", "and keeps one decimal")

    expect(PlanSummary.duration(0), "—", "no flight time means no duration")
    expect(PlanSummary.duration(.infinity), "—", "an infinite estimate is not shown")
    expect(PlanSummary.duration(100.079), "1:40", "minutes and seconds, zero padded")
    expect(PlanSummary.duration(9), "0:09", "under a minute still shows the minute")
    expect(PlanSummary.duration(3661), "1:01:01", "past an hour the hour appears")

    let flight = PlanSummary(distanceMetres: 500.4, seconds: 100.1, maxTelemetryMetres: 457.3)
    expect(flight.hasFlight, "a plan with distance and time has a flight to describe")
    expect(flight.distanceText, "500 m", "the summary formats its own distance")
    expect(flight.durationText, "1:40", "and its own duration")
    expect(flight.telemetryText, "457 m", "and the furthest it gets from launch")

    expect(!PlanSummary.empty.hasFlight, "an empty plan has nothing to summarise")
    expect(!PlanSummary(distanceMetres: 0, seconds: 0, maxTelemetryMetres: 0).hasFlight,
           "a launch point alone is not a flight")
}

checkPlanSummary()

func checkSurveyStats() {
    expect(!SurveyStats.none.describes, "an item that is not a survey describes nothing")

    let live = SurveyStats(shots: 1043, secondsBetweenShots: 0.8446969696969697,
                           areaSquareMetres: 89999.17662726832, footprintSide: 15.20,
                           footprintFrontal: 6.76, minimumInterval: 0)
    expect(live.describes, "a real survey does")
    expect(live.shotsText, "1043", "the photo count is whole")
    expect(live.intervalText, "0.84 s", "the interval is to a hundredth, as the camera is set")
    expect(live.areaText, "9.0 ha", "nine hectares reads as hectares, not ninety thousand metres")
    expect(live.footprintText, "15.2 \u{00D7} 6.8 m", "and each photo's ground footprint is given")
    expect(!live.tooFast, "a camera with no stated minimum is never too fast")

    expect(SurveyStats.area(500), "500 m\u{00B2}", "a small plot stays in square metres")
    expect(SurveyStats.area(10_000), "1.0 ha", "a hectare is the switch")
    expect(SurveyStats.area(2_500_000), "2.50 km\u{00B2}", "and a large one reads in square kilometres")
    expect(SurveyStats.area(0), "\u{2014}", "no area is not zero area")
    expect(SurveyStats.interval(0), "\u{2014}", "nor is no interval")

    let strained = SurveyStats(shots: 1043, secondsBetweenShots: 0.84, areaSquareMetres: 1,
                               footprintSide: 1, footprintFrontal: 1, minimumInterval: 2)
    expect(strained.tooFast, "a camera that needs two seconds cannot shoot every 0.84")
    expect(strained.warning.contains("2.00 s"), "and the warning names what the camera needs")
    expect(strained.warning.contains("0.84 s"), "alongside what the survey asks for")

    let stationary = SurveyStats(shots: 0, secondsBetweenShots: 0, areaSquareMetres: 0,
                                 footprintSide: 0, footprintFrontal: 0, minimumInterval: 2)
    expect(!stationary.tooFast, "a survey that takes no photos cannot outrun the camera")
}

checkSurveyStats()

func checkCalibration() {
    expect(!Calibration.read(["kind": "value"]).connected,
           "with no controller there is nothing to calibrate")

    expect(Calibration.property("UpsideDown", "InProgress"), "orientationCalUpsideDownSideInProgress",
           "the side properties are derived from the key, matching what the controller exposes")
    expect(Calibration.property("Down", "Done"), "orientationCalDownSideDone", "for every suffix")

    let idle = Calibration.read(["kind": "object", "calProgress": 0])
    expect(idle.connected, "an idle controller is still connected")
    expect(!idle.busy, "and not busy")
    expect(idle.visibleSides.isEmpty, "with no sides to show")
    expect(idle.needsAttention, "", "and nothing demanding attention")

    let needy = Calibration.read(["kind": "object", "accelSetupNeeded": true,
                                  "compassSetupNeeded": true])
    expect(needy.needsAttention, "The accelerometer and compass both need calibrating.",
           "both outstanding calibrations are named together")
    expect(Calibration.read(["kind": "object", "accelSetupNeeded": true]).needsAttention,
           "The accelerometer needs calibrating.", "and one on its own reads singly")

    let running: [String: Any] = [
        "kind": "object", "calibrationInProgress": true, "calProgress": 33.4,
        "showOrientationCalArea": true, "nextEnabled": true, "cancelEnabled": true,
        "orientationHelpText": "Hold still", "statusText": "Rotate the vehicle",
        "orientationCalDownSideVisible": true, "orientationCalDownSideDone": true,
        "orientationCalLeftSideVisible": true, "orientationCalLeftSideInProgress": true,
        "orientationCalLeftSideRotate": true,
        "orientationCalRightSideVisible": true,
    ]
    let live = Calibration.read(running)
    expect(live.busy, "a running calibration is busy")
    expect(live.progressText, "33%", "progress is whole percent")
    expect(live.visibleSides.map(\.title).joined(separator: ","), "Level,Left side,Right side",
           "only the sides this calibration asks for are shown, in the vehicle's order")
    expect(live.sides[0].stage == .done, "a finished side is done")
    expect(live.sides[2].stage == .inProgress, "the one being held is in progress")
    expect(live.sides[3].stage == .waiting, "and one not yet reached is waiting")
    expect(live.sides[2].symbol, "arrow.triangle.2.circlepath",
           "a side that must be rotated says so rather than showing a plain arrow")

    let cancelling = Calibration.read(["kind": "object", "waitingForCancel": true])
    expect(cancelling.busy, "a calibration being cancelled is still busy, so nothing else can start")

    expect(CalibrationRoutine.accelerometer.invocation, "sensorsCal.calibrateAccel",
           "each routine names the controller method it calls")
    expect(CalibrationRoutine.accelerometer.arguments.count == 1,
           "the accelerometer takes its simple-calibration flag")
    expect(CalibrationRoutine.gyro.arguments.isEmpty, "the others take none")
    expect(CalibrationRoutine.allCases.count == 5, "five routines are offered")
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
    expect(!Radio.read(["kind": "value"]).connected, "with no controller there is no radio page to fill")

    expect(Radio.property("throttle", "RCValue"), "throttleChannelRCValue",
           "stick properties are derived from the key, matching what the controller exposes")
    expect(Radio.property("roll", "Mapped"), "rollChannelMapped", "for every suffix")

    expect(RadioState.fraction(1000) == 0, "the low end of the travel is the left of the bar")
    expect(RadioState.fraction(2000) == 1, "and the high end fills it")
    expect(RadioState.fraction(1500) == 0.5, "centre sits in the middle")
    expect(RadioState.fraction(0) == 0, "a channel carrying nothing is empty, not centred")
    expect(RadioState.fraction(2500) == 1, "a value beyond the travel is clamped rather than overflowing")
    expect(RadioState.fraction(500) == 0, "at both ends")

    let live: [String: Any] = [
        "kind": "object", "channelCount": 16, "minChannelCount": 5,
        "rcValues": [1500, 1500, 1000, 1500, 1800, 1000, 1000, 1800, 0, 0],
        "rollChannelMapped": true, "rollChannelRCValue": 1500,
        "pitchChannelMapped": true, "pitchChannelRCValue": 1500,
        "yawChannelMapped": true, "yawChannelRCValue": 1500, "yawChannelReversed": 1,
        "throttleChannelMapped": true, "throttleChannelRCValue": 1000,
        "nextText": "Calibrate", "nextEnabled": true, "transmitterMode": 2,
    ]
    let state = Radio.read(live)
    expect(state.channelCount == 16, "the reported channel count is kept")
    expect(state.liveChannels.count == 8,
           "only channels carrying a signal are listed; the silent ones are not drawn as empty bars")
    expect(state.summary, "16 channels reported, 8 carrying a signal.", "and the summary says both")
    expect(state.enoughChannels, "sixteen is more than the five needed to fly")
    expect(state.shortfall, "", "so nothing is wanting")
    expect(!state.calibrating, "an idle controller is not calibrating")

    expect(state.sticks.map(\.title).joined(separator: ","), "Roll,Pitch,Yaw,Throttle",
           "the four sticks are named in the order a pilot reads them")
    expect(state.sticks[3].valueText, "1000", "throttle down reads as its pulse width")
    expect(state.sticks[2].reversed, "a reversed channel is marked")
    expect(!state.sticks[0].reversed, "and an unreversed one is not")

    let thin = Radio.read(["kind": "object", "channelCount": 4, "minChannelCount": 5])
    expect(!thin.enoughChannels, "four channels is not enough to fly")
    expect(thin.shortfall, "At least 5 channels are needed to fly; the transmitter reports 4.",
           "and the page says so in the pilot's terms")

    let silent = Radio.read(["kind": "object", "channelCount": 0, "minChannelCount": 5])
    expect(silent.summary, "No transmitter is being heard.",
           "a vehicle with no transmitter says that rather than reporting zero channels")
    expect(silent.shortfall, "", "and is not also scolded for having too few")

    let unmapped = Radio.sticks(from: ["kind": "object"])
    expect(unmapped[0].valueText, "Not mapped",
           "a stick with no channel assigned says so instead of showing a dash")
}

checkRadio()

func checkVideoStatus() {
    expect(!VideoStatus.read(["kind": "value"]).available,
           "a build with no video manager can show nothing")
    expect(VideoStatus.unavailable.summary, "This build cannot show video.",
           "and says so rather than claiming there is no stream")

    let live: [String: Any] = [
        "kind": "object", "hasVideo": true, "gstreamerEnabled": true, "isStreamSource": true,
        "decoding": false, "activeVideoSource": 0,
        "cameraStatuses": ["Connecting\u{2026}", "No stream URL", "Connecting\u{2026}", "No stream URL"],
        "cameraConnecting": [true, false, true, false],
        "cameraRecording": [false, false, false, false],
    ]
    let status = VideoStatus.read(live)
    expect(status.available, "a manager reporting video is available")
    expect(status.cameras.count == 4, "every slot the manager reports is read")
    expect(status.configuredCameras.count == 2,
           "a slot with no URL is not a camera anyone configured, so it is not listed")
    expect(status.configuredCameras.map(\.title).joined(separator: ","), "Camera 1,Camera 3",
           "and the ones that are keep their own slot numbers")
    expect(status.anyConnecting, "a slot still connecting is waiting for a stream")
    expect(status.summary, "Waiting for a stream.", "which is what the operator is told")
    expect(!status.settled, "and nothing is settled until something decodes")

    var decoding = VideoStatus.read(live)
    decoding.decoding = true
    expect(decoding.summary, "Streaming.", "once it decodes it is streaming")
    decoding.recording = true
    expect(decoding.summary, "Streaming and recording.", "and says when it is also recording")

    let idle = VideoStatus.read(["kind": "object", "hasVideo": true,
                                 "cameraStatuses": ["No stream URL"],
                                 "cameraConnecting": [false]])
    expect(idle.summary, "No stream URL is set.",
           "a manager with nothing configured says that, not that it failed")

    let short = VideoStatus.cameras(statuses: ["A", "B", "C"], connecting: [true], recording: [])
    expect(short.count == 3, "a status list longer than its flags still yields every camera")
    expect(short[0].connecting, "the flags that exist are used")
    expect(!short[2].connecting, "and the ones that do not default to false rather than crashing")
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
    expect(!CameraControl.read(["kind": "value"]).present, "no camera object means no camera")
    expect(!CameraControl.read(["kind": "object", "modelName": ""]).present,
           "nor does an object that will not name itself")

    let live: [String: Any] = [
        "kind": "object", "modelName": "Simulated Camera", "vendor": "QGroundControl",
        "cameraMode": "CAM_MODE_UNDEFINED", "photoCaptureStatus": "PHOTO_CAPTURE_IDLE",
        "videoCaptureStatus": "VIDEO_CAPTURE_STATUS_STOPPED", "recordTimeStr": "00:00:00",
        "storageStatus": "STORAGE_NOT_SUPPORTED", "storageFreeStr": "",
        "capturesPhotos": true, "capturesVideo": true, "hasModes": true, "batteryRemaining": -1,
    ]
    let camera = CameraControl.read(live)
    expect(camera.present, "the live SITL camera is present")
    expect(camera.title, "Simulated Camera", "and is named by its model")
    expect(camera.stateText, "Idle", "an idle camera is idle")
    expect(camera.modeText, "Not set", "an undefined mode is not guessed at")
    expect(!camera.modeKnown, "and is known to be unknown, so the page can say so")
    expect(camera.storageText, "Not reported",
           "a camera that does not support storage reporting says that, not zero bytes")
    expect(camera.batteryText, "", "a camera with no battery reading shows nothing at all")

    var recording = camera
    recording.videoStatus = CameraControl.recording
    recording.recordTime = "00:01:24"
    expect(recording.isRecording, "a running video capture is recording")
    expect(recording.stateText, "Recording 00:01:24", "and shows how long it has been going")

    var shooting = camera
    shooting.photoStatus = CameraControl.takingPhoto
    expect(shooting.stateText, "Taking a photo", "a photo in progress says so")

    var photoMode = camera
    photoMode.mode = CameraControl.photoMode
    expect(photoMode.modeText, "Photo", "a set mode is named")
    expect(photoMode.canPhoto, "and photo mode can take photos")
    expect(!photoMode.canRecord, "but not record video")

    var videoMode = camera
    videoMode.mode = CameraControl.videoMode
    expect(videoMode.canRecord, "video mode can record")
    expect(!videoMode.canPhoto, "but not shoot stills")

    var modeless = camera
    modeless.hasModes = false
    modeless.mode = CameraControl.undefinedMode
    expect(modeless.canPhoto, "a camera with no modes can do both")
    expect(modeless.canRecord, "without being told which it is in")

    let named = CameraControl.read(["kind": "object", "modelName": "Sim", "vendor": "QGC",
                                    "storageStatus": "STORAGE_READY", "storageFreeStr": "1.2 GB"])
    expect(named.storageText, "1.2 GB", "a camera that reports storage shows what is free")
}

checkCameraControl()

func checkLogReplayLink() {
    let empty = LinkConfig(index: 0, json: ["linkType": "TypeLogReplay", "name": "Replay"])
    expect(empty.editing == .logFile, "a log replay link is edited by choosing a file")
    expect(empty.displaySummary, "No log chosen",
           "and says so rather than showing an empty summary")

    let chosen = LinkConfig(index: 0, json: ["linkType": "TypeLogReplay", "name": "Replay",
                                             "filename": "/Users/pilot/logs/flight.tlog",
                                             "summary": "Log Replay"])
    expect(chosen.logFileName, "flight.tlog", "the row shows the log's name, not its whole path")
    expect(chosen.displaySummary, "Log Replay", "and the summary is left to the link once set")

    let tcp = LinkConfig(index: 0, json: ["linkType": "TypeTcp", "host": "1.2.3.4"])
    expect(tcp.editing == .hostAndPort, "other link types are unaffected")
}

checkLogReplayLink()

func checkMissionVehicle() {
    let copter = MissionVehicle(firmware: "ArduPilot", type: "Quadrotor", multiRotor: true, vtol: false)
    expect(copter.showsHoverSpeed, "a multirotor hovers between waypoints")
    expect(!copter.showsCruiseSpeed, "and never cruises, so asking a cruise speed would be noise")

    let plane = MissionVehicle(firmware: "PX4 Pro", type: "Fixed Wing", multiRotor: false, vtol: false)
    expect(plane.showsCruiseSpeed, "a plane cruises")
    expect(!plane.showsHoverSpeed, "and cannot hover")

    let vtol = MissionVehicle(firmware: "PX4 Pro", type: "VTOL", multiRotor: false, vtol: true)
    expect(vtol.showsCruiseSpeed, "a VTOL does both")
    expect(vtol.showsHoverSpeed, "so it needs both speeds")

    expect(copter.isDescribed, "a named firmware and type describe the vehicle")
    expect(!MissionVehicle.unknown.isDescribed, "an unknown vehicle describes nothing")
    expect(MissionVehicle.unknown.showsAnything,
           "even unknown, a cruise speed is still worth asking for the time estimate")
}

checkMissionVehicle()

func checkEraseWarning() {
    expect(LogEntry.eraseWarning(1).contains("The one log"),
           "one log does not read as \"All 1 log\"")
    expect(LogEntry.eraseWarning(3).contains("All 3 logs"), "several logs are counted")
    expect(LogEntry.eraseWarning(1).contains("gone for good"), "the warning says the loss is permanent")
    expect(LogEntry.eraseWarning(3).contains("gone for good"), "however many there are")
}

checkEraseWarning()

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
    func rate(_ hz: Double) -> String {
        MavlinkMessage(json: ["name": "ATTITUDE", "id": 30, "actualRateHz": hz], index: 0)?.rateText ?? "?"
    }

    expect(rate(10), "10.0 Hz", "a steady message reads in hertz")
    expect(rate(0), "—", "one that has never repeated has no rate")
    expect(rate(0.01), "<0.1 Hz", "a rare one is rare, not zero")
    expect(rate(3.04), "3.0 Hz", "the rate is rounded to a tenth")

    expect(MavlinkMessage(json: ["id": 30], index: 0) == nil, "a message without a name is not one")
    expect(MavlinkField(json: ["name": "roll", "type": "float", "value": "-0.01"])?.value ?? "", "-0.01",
           "a field carries the value the vehicle sent, unrounded")
    expect(MavlinkField(json: ["type": "float"]) == nil, "a field without a name is not one")
}

checkMavlinkMessage()

func checkPowerSections() {
    let batteryOne = Set(["BATT_MONITOR", "BATT_CAPACITY", "BATT_VOLT_MULT", "BATT2_MONITOR"])
    let present = SetupSection.present(SetupSection.power, in: batteryOne)
    expect(present.count == 2, "both packs appear when each has at least one parameter")
    expect(present[0].names.joined(separator: ","), "BATT_MONITOR,BATT_CAPACITY,BATT_VOLT_MULT",
           "only the parameters this firmware reports, in the page's order")
    expect(present[1].names.joined(separator: ","), "BATT2_MONITOR",
           "a second pack shows just its monitor until one is chosen")

    expect(SetupSection.present(SetupSection.power, in: []).isEmpty,
           "a vehicle reporting no battery parameters gets no sections at all")

    expect(BatteryReading.unavailable.available == false, "no voltage means no battery to show")
    let live = BatteryReading(voltage: 12.6, current: 1.5, percent: 100)
    expect(live.available, "a voltage is a battery")
    expect(live.voltageText, "12.60 V", "voltage reads to a hundredth, as a meter does")
    expect(live.currentText, "1.50 A", "so does current")
    expect(live.percentText, "100%", "the charge is whole percent")
    expect(BatteryReading(voltage: 12, current: nil, percent: nil).currentText, "—",
           "a pack measured for voltage only says nothing about current")
    expect(BatteryReading(voltage: .nan, current: nil, percent: nil).voltageText, "—",
           "an unset reading is not a number")
}

checkPowerSections()

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

    let present = SetupSection.present(SetupSection.frame, in: ["FRAME_CLASS"])
    expect(present.count == 1, "the airframe section survives a firmware with no FRAME_TYPE")
    expect(present[0].names.joined(separator: ","), "FRAME_CLASS", "showing only what is there")
}

checkFrameSetup()

func checkTuningSections() {
    let everything = Set(SetupSection.tuning.flatMap(\.parameters))
    let all = SetupSection.present(SetupSection.tuning, in: everything)
    expect(all.count == SetupSection.tuning.count, "a full firmware shows every tuning section")
    expect(all.map(\.section.title).joined(separator: ","),
           "Angle gains,Rate gains,Climb,Stick feel,Motor thrust",
           "in the order a tuner works through them")

    let ratesOnly = SetupSection.present(SetupSection.tuning, in: ["ATC_RAT_YAW_P", "PSC_ACCZ_P"])
    expect(ratesOnly.map(\.section.title).joined(separator: ","), "Rate gains,Climb",
           "sections with nothing present are dropped, not shown empty")
    expect(ratesOnly[0].names.joined(separator: ","), "ATC_RAT_YAW_P",
           "and each keeps only the parameters this firmware has")

    expect(everything.contains("MOT_THST_HOVER"), "hover thrust is part of tuning, not of power")
    expect(!everything.contains("AUTOTUNE_AXES"), "autotune is flown, so it is not on this page")
}

checkTuningSections()

func checkCameraSections() {
    let bare = SetupSection.present(SetupSection.camera,
                                    in: ["MNT1_TYPE", "MNT2_TYPE", "CAM1_TYPE", "CAM2_TYPE",
                                         "CAM_AUTO_ONLY", "CAM_MAX_ROLL", "CAM_RC_TYPE"])
    expect(bare.map(\.section.title).joined(separator: ","),
           "Gimbal 1,Gimbal 2,Camera 1,Camera 2,Triggering",
           "each mount and camera gets its own heading, so two rows never share a title")

    let oneOfEach = SetupSection.present(SetupSection.camera,
                                         in: ["MNT1_TYPE", "CAM1_TYPE", "CAM_MAX_ROLL"])
    expect(oneOfEach.map(\.section.title).joined(separator: ","), "Gimbal 1,Camera 1,Triggering",
           "a vehicle with one of each is not offered a second slot it does not have")

    let noGimbal = SetupSection.present(SetupSection.camera, in: ["CAM_MAX_ROLL"])
    expect(noGimbal.map(\.section.title).joined(separator: ","), "Triggering",
           "a vehicle with no mount at all drops the gimbal and camera sections")

    let names = Set(SetupSection.camera.flatMap(\.parameters))
    expect(!names.contains("MNT_RETRACT_X"),
           "the pre-4.2 MNT_ names are gone from ArduPilot and are not listed here")
}

checkCameraSections()

func checkVehicleMessages() {
    let live = "<font style=\"<#E>\">[23:24:17.897 ] Critical: PreArm: GPS 1: not healthy</font><br/>"
        + "<font style=\"<#N>\">[23:23:26.733 ] Info: Frame: QUAD/PLUS</font><br/>"
        + "<font style=\"<#I>\">[23:23:26.733 ] Warning: something odd</font><br/>"

    let parsed = VehicleMessage.parse(live)
    expect(parsed.count == 3, "every message in the run is read, newest first")
    expect(parsed[0].text, "PreArm: GPS 1: not healthy",
           "the tags and the severity word go; the dot already says the level")
    expect(VehicleMessage.withoutSeverity("PreArm: GPS 1: not healthy"), "PreArm: GPS 1: not healthy",
           "a message whose first word merely ends in a colon keeps all of it")
    expect(VehicleMessage.withoutSeverity("EMERGENCY: falling"), "falling",
           "every severity QGC emits is stripped, not just the common ones")
    expect(VehicleMessage.withoutSeverity("no colon at all"), "no colon at all",
           "and a message without a colon is untouched")
    expect(parsed[0].time, "23:24:17", "the stamp keeps the clock and drops the milliseconds")
    expect(parsed[0].level == .error, "a critical message is an error")
    expect(parsed[1].level == .normal, "an info message is not")
    expect(parsed[2].level == .warning, "and a warning is its own level")

    expect(VehicleMessage.parse("").isEmpty, "a vehicle that has said nothing has no messages")
    expect(VehicleMessage.parse("<font style=\"<#N>\"></font><br/>").isEmpty,
           "an empty message is dropped rather than shown blank")

    let bare = VehicleMessage.parse("<font style=\"<#N>\">no stamp here</font><br/>")
    expect(bare.count == 1, "a message without a stamp is still a message")
    expect(bare[0].text, "no stamp here", "and keeps all of its text")
    expect(VehicleMessage.parse("<font style=\"<#N>\">Info: unstamped</font><br/>")[0].text, "unstamped",
           "a message with no stamp still loses its severity word")
    expect(bare[0].time, "", "with no time to show")

    expect(VehicleMessage.worst(parsed) == .error, "the worst of a run is what the panel reports")
    expect(VehicleMessage.worst([]) == .normal, "silence is not a fault")
    expect(VehicleMessage.worst(Array(parsed.dropFirst())) == .warning,
           "without the error, a warning is the worst")
}

checkVehicleMessages()

func checkPreflight() {
    expect(Preflight.gps(lock: 6, satellites: 10).verdict == .passing,
           "an RTK fix with ten satellites passes")
    expect(Preflight.gps(lock: 2, satellites: 20).verdict == .failing("Waiting for 3D lock."),
           "satellites do not make up for a missing 3D lock")
    expect(Preflight.gps(lock: 3, satellites: 5).verdict
               == .overridable("Only 5 satellites; 9 wanted."),
           "a thin constellation is the operator's call, not a block")
    expect(Preflight.gps(lock: 3, satellites: 1).verdict
               == .overridable("Only 1 satellite; 9 wanted."), "and one satellite is singular")
    expect(Preflight.gps(lock: nil, satellites: nil).blocked, "no vehicle blocks the GPS check")

    expect(Preflight.battery(percent: 100).verdict == .passing, "a full pack passes")
    expect(Preflight.battery(percent: 39).blocked, "below forty percent blocks; it cannot be waved through")
    expect(Preflight.battery(percent: 40).verdict == .passing, "exactly forty is allowed")
    expect(Preflight.battery(percent: nil).blocked, "no battery reading blocks too")

    expect(Preflight.sensors(unhealthyBits: 0).verdict == .passing, "no unhealthy bits passes")
    expect(Preflight.sensors(unhealthyBits: 268435456).verdict == .passing,
           "a bit outside the mask is not one of the sensors this check covers")
    expect(Preflight.sensors(unhealthyBits: 268435488).verdict == .failing("GPS unhealthy."),
           "the live SITL case: GPS unhealthy alongside a bit we ignore")
    expect(Preflight.sensors(unhealthyBits: 3).verdict == .failing("Gyro, Accelerometer unhealthy."),
           "several sensors are all named")

    expect(Preflight.sound(muted: false).verdict == .passing, "audible QGC passes the sound check")
    expect(Preflight.sound(muted: true).blocked, "a muted QGC blocks it; warnings would go unheard")

    let list2 = Preflight.groups(airframe: .rover, lock: 6, satellites: 10, batteryPercent: 100,
                                 unhealthyBits: 0, audioMuted: false)

    func list(_ airframe: PreflightAirframe) -> [String] {
        Preflight.groups(airframe: airframe, lock: 6, satellites: 10, batteryPercent: 100,
                         unhealthyBits: 0, audioMuted: false).flatMap(\.checks).map(\.name)
    }

    let groups = Preflight.groups(airframe: .multiRotor, lock: 6, satellites: 10,
                                  batteryPercent: 100, unhealthyBits: 0, audioMuted: false)
    expect(Preflight.total(groups) == 11, "the multirotor list is eleven checks long")
    expect(Preflight.progress(groups, ticked: []), "0 of 11 checked", "and starts at none")
    expect(!Preflight.ready(groups, ticked: []), "an untouched list is not ready")

    let every = Set(groups.flatMap(\.checks).map(\.name))
    expect(Preflight.ready(groups, ticked: every), "ticking every check is ready")
    expect(Preflight.progress(groups, ticked: every), "11 of 11 checked", "and says so")
    expect(!Preflight.ready(groups, ticked: every.subtracting(["Payload"])),
           "one missing check is not ready")

    expect(PreflightAirframe.of(multiRotor: false, vtol: true, rover: false, sub: false,
                                fixedWing: true) == .vtol,
           "a VTOL also reports fixedWing; the VTOL list wins")
    expect(PreflightAirframe.of(multiRotor: false, vtol: false, rover: false, sub: false,
                                fixedWing: false) == .generic,
           "an airframe that claims nothing gets the generic list")

    expect(!list(.multiRotor).contains("Actuators"),
           "a multirotor has no control surfaces to sweep")
    expect(list(.fixedWing).contains("Actuators"), "a fixed wing does")
    expect(!list(.rover).contains("Motors"),
           "a rover is not asked to throttle its props up")
    expect(list(.rover).contains("Mission area") && !list(.rover).contains("Flight area"),
           "a rover drives a mission area, it does not launch into a flight area")
    expect(!list(.sub).contains("Wind and weather") && !list(.sub).contains("Flight area"),
           "wind and a launch area mean nothing underwater")
    expect(list(.sub).contains("Payload"), "a submarine still carries a payload")
    expect(list(.multiRotor).contains("Radio control") && list(.multiRotor).contains("Sound output"),
           "every list asks about the radio link and QGC's audio")

    expect(Preflight.progress(list2, ticked: ["Motors", "Payload"]), "1 of 10 checked",
           "a tick left over from another airframe is not counted against a list it is not on")

    expect(PreflightAirframe.multiRotor.hardwarePrompt, "Props mounted and secured?",
           "the hardware prompt names what this airframe actually has")
    expect(PreflightAirframe.sub.hardwarePrompt, "All seals in place?",
           "and a submarine is asked about its seals, not its props")
}

checkPreflight()

func checkVehicleWarning() {
    expect(!VehicleWarning.none.showing, "nothing wrong shows nothing")

    let disconnected = VehicleWarning.assess(
        connected: false, requiresGpsFix: true, hasCoordinate: false, armed: false,
        prearmError: "PreArm: GPS 1: not healthy", healthReportSupported: false)
    expect(!disconnected.showing, "with no vehicle there is nothing to warn about")

    let live = VehicleWarning.assess(
        connected: true, requiresGpsFix: true, hasCoordinate: true, armed: false,
        prearmError: "PreArm: GPS 1: not healthy", healthReportSupported: false)
    expect(live.lines.joined(separator: "|"), "PreArm: GPS 1: not healthy",
           "the live SITL case: a prearm refusal is shown as the vehicle worded it")

    let flying = VehicleWarning.assess(
        connected: true, requiresGpsFix: true, hasCoordinate: true, armed: true,
        prearmError: "PreArm: GPS 1: not healthy", healthReportSupported: false)
    expect(!flying.showing, "an armed vehicle is past prearm, so the refusal is stale")

    let px4 = VehicleWarning.assess(
        connected: true, requiresGpsFix: true, hasCoordinate: true, armed: false,
        prearmError: "PreArm: GPS 1: not healthy", healthReportSupported: true)
    expect(!px4.showing, "firmware with its own arming report owns the message instead")

    let lost = VehicleWarning.assess(
        connected: true, requiresGpsFix: true, hasCoordinate: false, armed: false,
        prearmError: "", healthReportSupported: false)
    expect(lost.lines.joined(separator: "|"), VehicleWarning.noGpsLockText,
           "a vehicle that needs GPS and has no position says so")

    let both = VehicleWarning.assess(
        connected: true, requiresGpsFix: true, hasCoordinate: false, armed: false,
        prearmError: "PreArm: GPS 1: not healthy", healthReportSupported: false)
    expect(both.lines.count == 2, "both faults are listed rather than one hiding the other")

    let noGpsNeeded = VehicleWarning.assess(
        connected: true, requiresGpsFix: false, hasCoordinate: false, armed: false,
        prearmError: "", healthReportSupported: false)
    expect(!noGpsNeeded.showing, "a vehicle that needs no GPS is not missing one")
}

checkVehicleWarning()

func checkInstrumentValues() {
    let facts: [[String: Any]] = [
        ["name": "altitudeRelative", "valueString": "12.3", "units": "m", "shortDescription": "Alt (Rel)"],
        ["name": "airSpeedSetpoint", "valueString": "0.024", "units": "", "shortDescription": ""],
    ]

    let altitude = InstrumentValue.resolve(.vehicle("altitudeRelative"), in: facts)
    expect(altitude.label, "Alt (Rel)", "the vehicle's own description names the reading")
    expect(altitude.value, "12.3", "and its formatted value is shown as the vehicle formats it")
    expect(altitude.units, "m", "with the units beside it")
    expect(!altitude.missing, "a fact that is present is not missing")

    let undescribed = InstrumentValue.resolve(.vehicle("airSpeedSetpoint"), in: facts)
    expect(undescribed.label, "Air Speed Setpoint",
           "a fact with no description falls back to its name, split into words")

    let absent = InstrumentValue.resolve(.vehicle("nothingHere"), in: facts)
    expect(absent.missing, "a fact this firmware does not report reads as missing")
    expect(absent.label, "Nothing Here", "but is still named, so the slot is not blank")

    expect(InstrumentValue.label(for: "gps"), "Gps", "a single lowercase word is capitalised")
    expect(InstrumentValue.label(for: "altitudeAMSL"), "Altitude A M S L",
           "runs of capitals split; the vehicle's own description is preferred for a reason")

    expect(InstrumentSelection.vehicle("heading").path, "vehicle",
           "the vehicle's own facts come from the vehicle itself")
    expect(InstrumentSelection(group: "gps", factName: "count").path, "vehicle.gps",
           "a named group is a child of it")
    expect(InstrumentSelection.defaults.count == 6, "six readings are shown before anyone configures it")
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

    let groups = InstrumentGroup.assemble(read)
    expect(groups.map(\.title).joined(separator: ","), "Vehicle,Gps,Battery 1",
           "a child carrying no facts is dropped, so rpm never reaches the picker")
    expect(groups[0].facts.map(\.label).joined(separator: ","), "Heading,Air Speed Setpoint",
           "a fact with no description falls back to its split name")
    expect(groups[0].facts.map(\.name).joined(separator: ","), "heading,airSpeedSetpoint",
           "while the name stays exactly as the vehicle reports it")

    expect(InstrumentGroup.title(for: ""), "Vehicle", "the vehicle's own facts are titled for it")
    expect(InstrumentGroup.title(for: "batteries.0"), "Battery 1",
           "an indexed battery reads as a battery, not as a path")
    expect(InstrumentGroup.title(for: "estimatorStatus"), "Estimator Status",
           "and a plain group is split into words")

    expect(InstrumentGroup.assemble([("gps", ["facts": [["shortDescription": "no name"]]])]).isEmpty,
           "a fact with no name is not offered, so an empty group is dropped entirely")

    let aliased: [(group: String, json: [String: Any])] = [
        ("", ["facts": [["name": "heading"]]]),
        ("vehicle", ["facts": [["name": "heading"]]]),
        ("gps", ["facts": [["name": "lat"], ["name": "lon"]]]),
        ("gps2", ["facts": [["name": "lat"], ["name": "lon"]]]),
    ]
    expect(InstrumentGroup.assemble(aliased).map(\.title).joined(separator: ","), "Vehicle,Gps,Gps2",
           "the vehicle lists itself among its children, so that one alias goes; two GPS units share a schema and both stay")
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

func checkGuidedActions() {
    func names(_ state: GuidedState) -> String {
        GuidedAction.available(in: state).map(\.rawValue).joined(separator: ",")
    }
    func offers(_ state: GuidedState, _ action: GuidedAction) -> Bool {
        GuidedAction.available(in: state).contains(action)
    }

    expect(names(GuidedState()), "", "with no vehicle nothing can be commanded")

    var idle = GuidedState()
    idle.connected = true
    idle.guidedSupported = true
    idle.takeoffSupported = true
    idle.pauseSupported = true
    idle.flightMode = "Guided"
    idle.rtlMode = "RTL"
    idle.landMode = "Land"
    idle.missionMode = "Auto"
    expect(names(idle), "", "a vehicle that will not arm is offered nothing to do")

    expect(GuidedAction.offered(in: idle).map(\.rawValue).joined(separator: ","), "arm,takeoff",
           "the actions are still shown while prearm fails, so the operator can see they exist")
    expect(GuidedAction.arm.offer(in: idle) == .blocked(GuidedAction.prearmReason),
           "and say why they cannot be used")

    idle.readyToArm = true
    expect(names(idle), "arm,takeoff", "once it can arm, arming and taking off are offered")
    expect(GuidedAction.arm.offer(in: idle) == .ready, "and are ready")

    var withMission = idle
    withMission.missionAvailable = true
    withMission.missionItemCount = 3
    expect(names(withMission), "arm,takeoff,startMission",
           "a loaded mission adds starting it, but only on the ground")

    var armed = withMission
    armed.armed = true
    expect(names(armed), "takeoff,startMission,land,disarm,emergencyStop",
           "an armed vehicle on the ground can be disarmed and stopped, and is not offered arming again")

    var flying = armed
    flying.flying = true
    expect(names(flying), "continueMission,pause,changeAltitude,land,rtl,emergencyStop",
           "in the air it can continue, hold, climb, land, return or be stopped, but never disarmed")

    var approaching = flying
    approaching.fixedWing = true
    approaching.landing = true
    expect(names(approaching).contains("landAbort"),
           "a fixed wing on approach is offered the abort")
    expect(!names(approaching).contains("pause"),
           "and holding position is withdrawn while it is on approach, as QGC does")
    expect(!names(flying).contains("landAbort"),
           "a multirotor in the cruise is never offered a landing abort")

    var landingMultiRotor = flying
    landingMultiRotor.landing = true
    expect(!names(landingMultiRotor).contains("landAbort"),
           "nor is a multirotor that is landing; the abort is a fixed-wing manoeuvre")

    expect(!names(flying).contains("changeSpeed"),
           "changing speed is withheld until the vehicle has reported its speed limits")
    var withLimits = flying
    withLimits.speedLimitsAvailable = true
    expect(names(withLimits).contains("changeSpeed"), "and offered once it has")

    var onMission = withLimits
    onMission.flightMode = onMission.missionMode
    expect(!names(onMission).contains("changeAltitude")
           && !names(onMission).contains("changeSpeed"),
           "neither is offered while the vehicle is flying its mission")

    var flyingUnready = flying
    flyingUnready.readyToArm = false
    expect(names(flyingUnready), "continueMission,pause,changeAltitude,land,rtl,emergencyStop",
           "a failing prearm never withholds getting a flying vehicle back down")

    var returning = flying
    returning.flightMode = "RTL"
    expect(!offers(returning, .rtl), "a vehicle already returning is not offered return")
    expect(returning.missionActive, "and counts as flying a mission")
    expect(!offers(returning, .continueMission), "so continuing is withheld")

    var landing = flying
    landing.flightMode = "Land"
    expect(!offers(landing, .land), "a vehicle already landing is not offered land")

    var plane = flying
    plane.fixedWing = true
    expect(!offers(plane, .land), "a fixed wing is not offered a guided land")
    expect(offers(plane, .rtl), "but can still be sent home")

    var midMission = flying
    midMission.currentMissionIndex = 1
    expect(offers(midMission, .continueMission),
           "with items left, the rest of the mission can be continued")
    midMission.currentMissionIndex = 2
    expect(!offers(midMission, .continueMission),
           "at the last item there is nothing left to continue")

    expect(GuidedAction.emergencyStop.destructive, "the emergency stop is marked destructive")
    expect(!GuidedAction.rtl.destructive, "returning home is not")
}

checkGuidedActions()

if failures == 0 {
    print("all Swift checks passed")
    exit(0)
}
FileHandle.standardError.write("\(failures) check(s) failed\n".data(using: .utf8)!)
exit(1)
