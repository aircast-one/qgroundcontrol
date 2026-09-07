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

if failures == 0 {
    print("all Swift checks passed")
    exit(0)
}
FileHandle.standardError.write("\(failures) check(s) failed\n".data(using: .utf8)!)
exit(1)
