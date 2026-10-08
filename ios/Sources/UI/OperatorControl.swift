import Foundation

let OPERATOR_CONTROL_VIEW = "view.operatorControl"

struct ControlStation: Equatable {
    let known: Bool
    let inControl: Bool?
    let holderSystemId: Int?
    let takeoverAllowed: Bool?
    let requestAllowed: Bool
    let reason: String
}

func controlStation(_ view: JSON?) -> ControlStation? {
    guard let view, view["available"].bool else { return nil }
    return ControlStation(
        known: view["known"].bool,
        inControl: view["inControl"].isNull ? nil : view["inControl"].bool,
        holderSystemId: view["holderSystemId"].isNull ? nil : view["holderSystemId"].int(0),
        takeoverAllowed: view["takeoverAllowed"].isNull ? nil : view["takeoverAllowed"].bool,
        requestAllowed: view["requestAllowed"].bool,
        reason: view["reason"].string
    )
}

func controlIsElsewhere(_ station: ControlStation?) -> Bool { station?.inControl == false }

func controlLine(_ station: ControlStation?) -> String? {
    guard let station, station.known, station.inControl != true, !station.reason.isBlank else { return nil }
    return station.holderSystemId.map { "\(station.reason) (system \($0))" } ?? station.reason
}

func holderLine(_ station: ControlStation?) -> String? {
    guard let held = station, held.known, held.inControl == false else { return nil }
    return held.holderSystemId.map { "System in control: \($0)" } ?? (held.reason.isBlank ? nil : held.reason)
}

func acquireLabel(_ station: ControlStation?) -> String? {
    guard let station, station.known, station.inControl == false else { return nil }
    return station.takeoverAllowed == true ? "Acquire control" : "Send request"
}

func acquireEnabled(_ station: ControlStation, _ counting: Bool) -> Bool { station.requestAllowed && !counting }

func controlWaitLine(_ station: ControlStation?) -> String? {
    guard let station, station.known, station.inControl == false, !station.requestAllowed else { return nil }
    return "Waiting for the other station to answer"
}

func inControlLine(_ station: ControlStation?) -> String? {
    guard let held = station, held.known, held.inControl == true else { return nil }
    return held.holderSystemId.map { "System in control: This GCS (\($0))" } ?? "System in control: This GCS"
}

func takeoverLine(_ station: ControlStation?) -> String? {
    guard let station, station.known, let allowed = station.takeoverAllowed else { return nil }
    return allowed ? "Takeover allowed" : "Takeover NOT allowed"
}

func takeoverChangeable(_ station: ControlStation?, _ allowTakeover: Bool?) -> Bool {
    station?.inControl == true && allowTakeover != nil && station?.takeoverAllowed != allowTakeover
}

func requestSentLabel(_ remainingMs: Int64) -> String {
    "Request sent: \(String(format: "%.1f", Double(max(remainingMs, 0)) / 1000.0))"
}

func allowTakeoverSetting() -> Bool? {
    let read = Qgc.get(ALLOW_TAKEOVER_SETTING)
    return read["value"].isNull ? nil : read["value"].bool
}

func saveAllowTakeover(_ allow: Bool) -> String? {
    Qgc.set("\(ALLOW_TAKEOVER_PATH).rawValue", allow) ? nil : "This station could not save whether it allows a takeover."
}

func changeTakeover(_ allow: Bool) -> String? {
    Qgc.invoke(REQUEST_CONTROL, allow, 0) ? nil : "The vehicle did not take the change."
}

func allowTakeoverEditable(_ station: ControlStation?) -> Bool {
    station?.inControl == true || station?.takeoverAllowed == true
}

func controlSectionTitle(_ station: ControlStation) -> String? {
    switch station.inControl {
    case true: "Change takeover condition"
    case false: "Send control request"
    case nil: nil
    }
}

func requestTimeoutEditable(_ station: ControlStation) -> Bool {
    station.inControl == false && station.takeoverAllowed != true
}

func requestTimeoutSeconds(_ station: ControlStation, _ setting: Int) -> Int {
    station.takeoverAllowed == true ? 0 : setting
}

let REQUEST_CONTROL = "vehicle.requestOperatorControl"
let ALLOW_TAKEOVER_PATH = "settings.flyViewSettings.requestControlAllowTakeover"
let ALLOW_TAKEOVER_SETTING = settingControl(ALLOW_TAKEOVER_PATH)
let REQUEST_TIMEOUT_PATH = "settings.flyViewSettings.requestControlTimeout"
let REQUEST_TIMEOUT_SETTING = settingControl(REQUEST_TIMEOUT_PATH)
let GCS_SYSTEM_ID_PATH = "settings.mavlinkSettings.gcsMavlinkSystemID"

struct ControlAsk: Equatable {
    let refusal: String?
    let timeoutSeconds: Int
}

func askForControl(_ station: ControlStation) -> ControlAsk {
    guard let allowTakeover = allowTakeoverSetting() else {
        return ControlAsk(refusal: "This station cannot tell whether it would allow a takeover, so it did not ask.", timeoutSeconds: 0)
    }
    let read = Qgc.get(REQUEST_TIMEOUT_SETTING)
    guard !read["value"].isNull else {
        return ControlAsk(refusal: "This station cannot tell how long it would wait, so it did not ask.", timeoutSeconds: 0)
    }
    let seconds = requestTimeoutSeconds(station, read["value"].int(0))
    return Qgc.invoke(REQUEST_CONTROL, allowTakeover, seconds)
        ? ControlAsk(refusal: nil, timeoutSeconds: seconds)
        : ControlAsk(refusal: "The vehicle did not take the request.", timeoutSeconds: 0)
}
