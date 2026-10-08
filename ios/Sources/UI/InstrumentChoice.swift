import Foundation

let INSTRUMENT_GROUPS = "view.instrumentGroups"

let DEFAULT_INSTRUMENTS = [
    "distanceToHome",
    "altitudeRelative",
    "groundSpeed",
    "climbRate",
]

private let FORWARD_FLIGHT_CLASSES: Set<String> = ["fixedWing", "vtol", "airship"]

func defaultInstruments(_ vehicleClass: String) -> [String] {
    DEFAULT_INSTRUMENTS + ["airSpeed"].filter { _ in FORWARD_FLIGHT_CLASSES.contains(vehicleClass) }
}

let INSTRUMENTS_VIEW = "view.instruments"

struct InstrumentFact: Equatable, Hashable {
    let name: String
    let label: String
    let path: String
}

struct InstrumentGroup: Equatable {
    let group: String
    let title: String
    let facts: [InstrumentFact]
}

func instrumentGroups(_ view: JSON?) -> [InstrumentGroup] {
    listedGroups(view, "groups") + listedGroups(view, "packGroups")
}

private func instrumentFacts(_ listed: [JSON]) -> [InstrumentFact] {
    listed
        .filter { $0.object != nil }
        .map { InstrumentFact(name: $0["name"].string, label: $0["label"].string, path: $0["selection"].string) }
        .filter { !$0.name.isBlank && !$0.path.isBlank }
}

private func listedGroups(_ view: JSON?, _ key: String) -> [InstrumentGroup] {
    guard let view, view["available"].bool, let listed = view[key].arrayOrNil else { return [] }
    return listed
        .filter { $0.object != nil }
        .map { InstrumentGroup(group: $0["group"].string, title: $0["title"].string, facts: instrumentFacts($0["facts"].array)) }
        .filter { !$0.facts.isEmpty }
}

func vehicleOwnGroup(_ view: JSON?) -> InstrumentGroup? {
    guard let view, view["available"].bool, let facts = view["vehicleFacts"].arrayOrNil else { return nil }
    let listed = instrumentFacts(facts)
    return listed.isEmpty ? nil : InstrumentGroup(group: "vehicle", title: "Vehicle", facts: listed)
}

func instrumentsPath(_ chosen: [String], vehicleClass: String = GENERIC_CLASS) -> String {
    chosen.isEmpty || chosen == defaultInstruments(vehicleClass)
        ? INSTRUMENTS_VIEW
        : "\(INSTRUMENTS_VIEW)(\(chosen.joined(separator: ",")))"
}

func showsInstruments(_ chosen: [String]) -> Bool { !chosen.isEmpty }

func withInstrument(_ chosen: [String], _ name: String) -> [String] {
    chosen.firstIndex(of: name).map { at in chosen.enumerated().filter { $0.offset != at }.map(\.element) } ?? chosen + [name]
}

func selectionId(_ selection: String) -> String { selection.contains("/") ? selection : "vehicle/\(selection)" }

func movedInstrument(_ chosen: [String], _ index: Int, _ by: Int) -> [String] {
    let to = index + by
    guard chosen.indices.contains(to), chosen.indices.contains(index) else { return chosen }
    return chosen.enumerated().map { i, name in i == index ? chosen[to] : i == to ? chosen[index] : name }
}

func replacedInstrument(_ chosen: [String], _ index: Int, _ path: String) -> [String] {
    chosen.enumerated()
        .map { $0.offset == index ? path : $0.element }
        .enumerated()
        .filter { $0.offset == index || $0.element != path }
        .map(\.element)
}

func removedInstrument(_ chosen: [String], _ index: Int) -> [String] {
    chosen.enumerated().filter { $0.offset != index }.map(\.element)
}

func emptyCatalogueText(_ connected: Bool) -> String {
    connected ? "This vehicle reported no readings this screen can ask for." : "Connect a vehicle to see what it can report."
}

func instrumentChoiceNote(_ chosen: [String]) -> String {
    chosen.isEmpty ? "Nothing chosen. The flight screen shows no readings." : "\(chosen.count) chosen."
}

private let STORE = "fly-instruments"
private let GENERIC_CLASS = "generic"
private let store = UserDefaults(suiteName: STORE) ?? .standard

func instrumentVehicleClass(_ view: JSON?) -> String {
    (view?["vehicleClass"].string ?? "").ifBlank(GENERIC_CLASS)
}

func chosenKey(_ vehicleClass: String) -> String { "chosen-\(vehicleClass)" }

func readChosen(_ vehicleClass: String) -> [String] {
    store.string(forKey: chosenKey(vehicleClass))
        .map { $0.components(separatedBy: ",").filter { !$0.isBlank } }
        ?? defaultInstruments(vehicleClass)
}

func writeChosen(_ vehicleClass: String, _ chosen: [String]) {
    store.set(chosen.joined(separator: ","), forKey: chosenKey(vehicleClass))
    offMain { AppCommands.setSubtitleInstruments(vehicleClass, chosen) }
}

private let VALUE_SIZE_KEY = "fontSize"

enum ValueSize: Int, CaseIterable {
    case Default, Small, Medium, Large

    var label: String {
        switch self {
        case .Default: "Default"
        case .Small: "Small"
        case .Medium: "Medium"
        case .Large: "Large"
        }
    }

    var scale: Double {
        switch self {
        case .Default: 1
        case .Small: 0.86
        case .Medium: 1.25
        case .Large: 1.5
        }
    }

    var ordinal: Int { rawValue }
}

func valueSizeAt(_ ordinal: Int) -> ValueSize { ValueSize(rawValue: ordinal) ?? .Default }

func nextValueSize(_ size: ValueSize) -> ValueSize { valueSizeAt((size.ordinal + 1) % ValueSize.allCases.count) }

func valueSizePillText(_ size: ValueSize) -> String { "Size: \(size.label)" }

func readValueSize(_ vehicleClass: String) -> ValueSize {
    valueSizeAt(store.integer(forKey: "\(VALUE_SIZE_KEY)-\(vehicleClass)"))
}

func writeValueSize(_ vehicleClass: String, _ size: ValueSize) {
    store.set(size.ordinal, forKey: "\(VALUE_SIZE_KEY)-\(vehicleClass)")
}

func shownFirst(_ groups: [InstrumentGroup], _ shown: [String]) -> [InstrumentGroup] {
    let all = groups.flatMap(\.facts)
    let picked = InstrumentGroup(group: SHOWN_GROUP, title: "Shown", facts: shown.compactMap { path in all.first { $0.path == path } })
    return ([picked] + groups.map { group in InstrumentGroup(group: group.group, title: group.title, facts: group.facts.filter { !shown.contains($0.path) }) })
        .filter { !$0.facts.isEmpty }
}

private let SHOWN_GROUP = "shown"
