import SwiftUI

let FLY_STATE = "view.flyState"

struct FlyState: Equatable {
    var connected: Bool
    var armed: Bool
    var contactLost: Bool?
    var state: String
    var stateText: String
    var staleNotice: String
    var mode: String
    var rcSupported: Bool
    var rcSignalText: String
    var rcSignal: Int?
    var rcOverride: Bool?
    var telemetry: TelemetryLink?
    var summaryDetail: String = ""
    var nominal: Bool = true
    var fault: Bool = false
    var canArm: Bool = true
    var readyToFly: Bool = true
}

enum ChipTone { case Error, Neutral, Warning, Success }

func notReadyToFly(_ state: FlyState?) -> Bool {
    guard let state else { return false }
    return state.connected && !state.armed && state.contactLost != true && !state.readyToFly
}

func chipTone(_ state: FlyState?, _ lost: Bool) -> ChipTone {
    if lost || state?.fault == true { return .Error }
    guard let state, state.connected else { return .Neutral }
    if !state.nominal { return .Warning }
    return notReadyToFly(state) ? .Neutral : .Success
}

let ALL_CHECKS_PASSED = "All checks passed."
let SETUP_NOT_COMPLETE = "Aircraft setup is not complete."

func readinessWarning(_ state: FlyState?) -> String? {
    guard let state, state.connected, !state.armed, notReadyToFly(state) || !state.nominal else { return nil }
    let detail = notReadyToFly(state) && state.summaryDetail == ALL_CHECKS_PASSED ? SETUP_NOT_COMPLETE : state.summaryDetail
    let text = [state.stateText, detail].filter { !$0.isBlank }.joined(separator: ". ")
    return text.isBlank ? nil : text
}

struct Readiness: Equatable {
    let text: String
    let blocks: Bool
}

let READINESS_BLOCKED = "The vehicle will refuse to arm until this is fixed."

func guidedReadiness(_ state: FlyState?) -> Readiness? {
    readinessWarning(state).map { text in
        let blocks = state?.canArm == false
        return Readiness(text: blocks ? "\(text.removingSuffix(".")). \(READINESS_BLOCKED)" : text, blocks: blocks)
    }
}

func disarmNotice(_ wasArmed: Bool, _ flewWhileArmed: Bool, _ armedNow: Bool) -> String? {
    wasArmed && !armedNow ? (flewWhileArmed ? "Landed and disarmed" : "Disarmed") : nil
}

let VEHICLE_FLIGHT_DISTANCE = "vehicle.flightDistance"

func flownDistanceText(_ view: JSON?) -> String? {
    guard let view, view["value"].double(0) > 0 else { return nil }
    let shown = view["valueString"].string
    return shown.isBlank ? nil : "\(shown) \(view["units"].string)".trimmed
}

func landedSummary(_ seconds: Double?, _ distance: String?, _ batteryUsed: Int?) -> String {
    [
        seconds.flatMap { $0 >= 1 ? flightTimeText($0) : nil },
        distance.flatMap { $0.isBlank ? nil : $0 },
        batteryUsed.flatMap { $0 > 0 ? "\($0)% battery used" : nil },
    ].compactMap { $0 }.joined(separator: " \u{00b7} ")
}

struct TelemetryLink: Equatable {
    var localRssiDbm: Int
    var remoteRssiDbm: Int?
    var localNoise: Int?
    var remoteNoise: Int?
    var receiveErrors: Int?
    var errorsFixed: Int? = nil
    var txBuffer: Int? = nil
}

private func optionalInt(_ value: JSON) -> Int? { value.isNull ? nil : value.int(0) }

func telemetryLink(_ view: JSON?) -> TelemetryLink? {
    guard let radio = view?["telemetry"], radio.object != nil, !radio["localRssiDbm"].isNull else { return nil }
    return TelemetryLink(
        localRssiDbm: radio["localRssiDbm"].int(0),
        remoteRssiDbm: optionalInt(radio["remoteRssiDbm"]),
        localNoise: optionalInt(radio["localNoise"]),
        remoteNoise: optionalInt(radio["remoteNoise"]),
        receiveErrors: optionalInt(radio["receiveErrors"]),
        errorsFixed: optionalInt(radio["errorsFixed"]),
        txBuffer: optionalInt(radio["txBuffer"])
    )
}

func flyState(_ view: JSON?) -> FlyState? {
    guard let view, view["class"].string == "FlyState" else { return nil }
    return FlyState(
        connected: view["connected"].bool,
        armed: view["armed"].bool,
        contactLost: view["contactLost"].isNull ? nil : view["contactLost"].bool,
        state: view["state"].string,
        stateText: view["stateText"].string.components(separatedBy: " · ").map(sentenceCase).joined(separator: " · "),
        staleNotice: view["staleNotice"].string,
        mode: view["mode"].string,
        rcSupported: view["rcSupported"].bool,
        rcSignalText: view["rcSignalText"].string,
        rcSignal: optionalInt(view["rcSignal"]),
        rcOverride: view["rcOverride"].isNull ? nil : view["rcOverride"].bool,
        telemetry: telemetryLink(view),
        summaryDetail: view["summaryDetail"].string,
        nominal: view["nominal"].bool(true),
        fault: view["fault"].bool,
        canArm: view["canArm"].bool(true),
        readyToFly: view["readyToFly"].bool(true)
    )
}

func offlineMainStatus(_ view: JSON?) -> String? {
    guard let status = view?["mainStatus"].string, !status.isBlank else { return nil }
    return sentenceCase(status)
}

func vehicleSubtitle(_ state: FlyState?, _ offline: String? = nil) -> String {
    guard let state, state.connected else { return offline ?? "No vehicle" }
    if state.contactLost == true { return state.stateText }
    return [state.mode.isBlank ? nil : state.mode, state.stateText].compactMap { $0 }.joined(separator: " · ")
}

@propertyWrapper
struct HasVehicle: DynamicProperty {
    @QgcPath(FLY_STATE) private var view

    var wrappedValue: Bool { flyState(view)?.connected == true }
}
