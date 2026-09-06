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

if failures == 0 {
    print("all Swift checks passed")
    exit(0)
}
FileHandle.standardError.write("\(failures) check(s) failed\n".data(using: .utf8)!)
exit(1)
