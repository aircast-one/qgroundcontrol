import Foundation
import os

private let SET_ACTIVE_VEHICLE = "vehicles.setActive"

enum VehicleBridge {
    private static let asked = OSAllocatedUnfairLock<(refusal: String?, id: Int?)>(initialState: (nil, nil))

    static var lastRefusal: String? { asked.withLock { $0.refusal } }

    static var lastAsked: Int? { asked.withLock { $0.id } }

    static func forget() {
        asked.withLock { $0.id = nil }
    }

    static func askFor(_ id: Int) -> Bool { answered(id, Qgc.call(SET_ACTIVE_VEHICLE, id)) }

    static func answered(_ id: Int, _ answer: JSON?) -> Bool {
        let accepted = answer?["ok"].bool == true
        let refusal = answer.map { $0["reason"].string } ?? "bridge threw: no answer"
        asked.withLock { state in
            state.refusal = refusal.nonBlank
            if accepted { state.id = id }
        }
        return accepted
    }
}

let VEHICLES_VIEW = "view.vehicles"

let VEHICLE_MANAGER = "vehicles"

enum FleetBridge {
    static func setSelected(_ id: Int, _ selected: Bool) -> Bool {
        invokeOk(selected ? "\(VEHICLE_MANAGER).selectVehicle" : "\(VEHICLE_MANAGER).deselectVehicle", id)
    }

    static func selectAll(_ choices: VehicleChoices) -> Bool {
        choices.choices.filter { !$0.selected }.map { setSelected($0.id, true) }.allSatisfy { $0 }
    }

    static func deselectAll() -> Bool { invokeOk("\(VEHICLE_MANAGER).deselectAllVehicles") }

    static func command(_ action: String, _ confirmed: Set<Int>) -> Bool {
        let targets = selectedPaths().filter { confirmed.contains($0.id) }
        guard !targets.isEmpty else { return false }
        switch action {
        case "mvArm":
            return targets.map { target in still(target) { setOk("\($0).armed", true) } }.allSatisfy { $0 }
        case "mvDisarm":
            return targets.map { target in still(target) { setOk("\($0).armed", false) } }.allSatisfy { $0 }
        case "mvPause":
            return targets.map { target in still(target) { invokeOk("\($0).pauseVehicle") } }.allSatisfy { $0 }
        case "mvStartMission":
            let sent = targets.filter { armedAt($0.path) }.map { target in still(target) { invokeOk("\($0).startMission") } }
            return !sent.isEmpty && sent.allSatisfy { $0 }
        default:
            return false
        }
    }

    private static func still(_ target: (path: String, id: Int), _ write: (String) -> Bool) -> Bool {
        idAt(target.path) == target.id && write(target.path)
    }

    private static func selectedPaths() -> [(path: String, id: Int)] {
        let count = Qgc.get("\(VEHICLE_MANAGER).selectedVehicles.count")["value"].int(0)
        return (0..<max(count, 0)).compactMap { index in
            let path = "\(VEHICLE_MANAGER).selectedVehicles.\(index)"
            return idAt(path).map { (path: path, id: $0) }
        }
    }

    private static func idAt(_ path: String) -> Int? {
        let id = Qgc.get(path, fields: ["id"])["id"].int(-1)
        return id >= 0 ? id : nil
    }

    private static func armedAt(_ path: String) -> Bool {
        Qgc.get(path, fields: ["armed"])["armed"].bool
    }
}

let CHOOSER_TITLE = "Fly which aircraft?"

struct VehicleChoice: Equatable, Identifiable {
    var id: Int
    var name: String
    var state: String
    var link: String
    var contactLost: Bool
    var active: Bool
    var latitude: Double = .nan
    var longitude: Double = .nan
    var selected: Bool = false
    var heading: Double = .nan
    var home: TrackPoint? = nil
    var radar: RadarReading? = nil
    var armed: Bool = false
    var telemetry: [(String, String)] = []
    var index: Int = -1
    var flightModes: [String] = []

    static func == (lhs: VehicleChoice, rhs: VehicleChoice) -> Bool {
        lhs.id == rhs.id && lhs.name == rhs.name && lhs.state == rhs.state && lhs.link == rhs.link
            && lhs.contactLost == rhs.contactLost && lhs.active == rhs.active
            && sameDouble(lhs.latitude, rhs.latitude) && sameDouble(lhs.longitude, rhs.longitude)
            && lhs.selected == rhs.selected && sameDouble(lhs.heading, rhs.heading)
            && lhs.home == rhs.home && lhs.radar == rhs.radar && lhs.armed == rhs.armed
            && lhs.telemetry.elementsEqual(rhs.telemetry) { $0.0 == $1.0 && $0.1 == $1.1 }
            && lhs.index == rhs.index && lhs.flightModes == rhs.flightModes
    }
}

func vehicleFlightModePath(_ choice: VehicleChoice) -> String { "vehicles.vehicles.\(choice.index).flightMode" }

func vehicleTelemetryLine(_ choice: VehicleChoice) -> String? {
    choice.telemetry.isEmpty ? nil : choice.telemetry.map { "\($0.0) \($0.1)" }.joined(separator: " \u{00b7} ")
}

struct VehicleChoices: Equatable {
    var ambiguous: Bool
    var choices: [VehicleChoice]
    var canSelectAll: Bool = false
    var canDeselectAll: Bool = false

    var active: VehicleChoice? { choices.first { $0.active } }
    var selectedCount: Int { choices.filter(\.selected).count }
}

func vehicleChoices(_ view: JSON?) -> VehicleChoices {
    VehicleChoices(
        ambiguous: view?["ambiguous"].bool == true,
        choices: (view?["vehicles"].objects ?? []).compactMap(vehicleChoice),
        canSelectAll: view?["canSelectAll"].bool == true,
        canDeselectAll: view?["canDeselectAll"].bool == true
    )
}

private func vehicleChoice(_ entry: JSON) -> VehicleChoice? {
    let id = entry["id"].int(-1)
    guard id >= 0 else { return nil }
    let home = entry["home"].objectOrNil.map { TrackPoint(latitude: $0["latitude"].double(.nan), longitude: $0["longitude"].double(.nan)) }
    return VehicleChoice(
        id: id,
        name: entry["name"].string.ifBlank("Vehicle \(id)"),
        state: vehicleChoiceState(entry),
        link: entry["link"].string,
        contactLost: entry["contactLost"].bool,
        active: entry["active"].bool,
        latitude: entry["coordinate"]["latitude"].double ?? .nan,
        longitude: entry["coordinate"]["longitude"].double ?? .nan,
        selected: entry["selected"].bool,
        heading: entry["heading"].isNull ? .nan : entry["heading"].double(.nan),
        home: home.flatMap { isPlottable($0.latitude, $0.longitude) ? $0 : nil },
        radar: radarReading(entry["proximity"].objectOrNil),
        armed: entry["armed"].bool,
        telemetry: entry["telemetry"].objects.map { ($0["label"].string, $0["value"].string) },
        index: entry["index"].int(-1),
        flightModes: entry["flightModes"].strings.filter { !$0.isBlank }
    )
}

private func vehicleChoiceState(_ entry: JSON) -> String {
    let flightMode = entry["flightMode"].string
    let armedState = entry["flying"].bool ? "Flying" : entry["armed"].bool ? "Armed" : "Disarmed"
    return [flightMode.nonBlank, armedState].compactMap { $0 }.joined(separator: " · ")
}

func linkDistinguishes(_ choices: [VehicleChoice]) -> Bool {
    Set(choices.map(\.link).filter { !$0.isBlank }).count > 1
}

func vehicleChoiceLine(_ choice: VehicleChoice, _ distinguishes: Bool = true) -> String {
    let link = distinguishes && !choice.link.isBlank ? choice.link : nil
    let lead = choice.contactLost ? "No contact" : choice.state.nonBlank
    return [lead, link].compactMap { $0 }.joined(separator: " · ")
}

func lostVehicles(_ choices: VehicleChoices) -> [VehicleChoice] {
    choices.choices.filter { $0.contactLost && !$0.active }
}

func lostVehiclesText(_ lost: [VehicleChoice]) -> String? {
    switch lost.count {
    case 0: nil
    case 1: "\(lost[0].name) is not answering"
    default: "\(lost.count) other vehicles are not answering"
    }
}

func activeVehicleTitle(_ choices: VehicleChoices, _ subtitle: String) -> String {
    guard choices.ambiguous else { return subtitle }
    return [choices.active?.name, subtitle.nonBlank].compactMap { $0 }.joined(separator: " · ")
}

func uploadHeading(_ gate: UploadGate, _ choices: VehicleChoices) -> String {
    let asked = gate.heading.ifBlank("Upload this plan?")
    guard choices.ambiguous, let target = choices.active?.name else { return asked }
    return asked.hasSuffix("?") ? "\(asked.dropLast()) to \(target)?" : "\(asked) to \(target)"
}

func handoverNotice(_ before: VehicleChoices?, _ now: VehicleChoices, _ asked: Int?) -> String? {
    guard let left = before?.active, let arrived = now.active, left.id != arrived.id, arrived.id != asked else { return nil }
    return now.choices.contains { $0.id == left.id }
        ? "\(left.name) stopped answering. Now flying \(arrived.name)."
        : "\(left.name) is gone. Now flying \(arrived.name)."
}

func activeChanged(_ before: VehicleChoices?, _ now: VehicleChoices) -> Bool {
    guard let was = before?.active?.id, let isNow = now.active?.id else { return false }
    return was != isNow
}

func rememberedChoices(_ previous: VehicleChoices?, _ now: VehicleChoices) -> VehicleChoices? {
    if now.choices.isEmpty { return nil }
    return now.active != nil ? now : previous
}
