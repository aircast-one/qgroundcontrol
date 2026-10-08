import Foundation

let FLIGHT_MODES = "view.flightModes"

struct FlightModeOption: Equatable, Hashable {
    var name: String
    var summary: String
    var current: Bool
    var needsConfirm: Bool
    var hidden: Bool
    var section: String = "normal"
}

struct FlightModesView: Equatable {
    var canSet: Bool
    var current: String
    var currentSummary: String
    var everyday: [FlightModeOption]
    var folded: [FlightModeOption]
    var all: [FlightModeOption]
    var hidden: [String]
    var hiddenSetting: String?
    var unknownModeNotice: String = ""
}

private func options(_ view: JSON?, _ key: String) -> [FlightModeOption] {
    guard let items = view?[key].arrayOrNil else { return [] }
    return items.filter { $0.object != nil }.map { mode in
        FlightModeOption(
            name: mode["name"].string,
            summary: mode["summary"].string,
            current: mode["current"].bool,
            needsConfirm: mode["needsConfirm"].bool,
            hidden: mode["hidden"].bool,
            section: mode["section"].string.ifBlank("normal")
        )
    }
}

func flightModesView(_ view: JSON?) -> FlightModesView? {
    guard let view, view["available"].bool else { return nil }
    let hiddenSetting = view["hiddenSetting"].string
    return FlightModesView(
        canSet: view["canSet"].bool,
        current: view["current"].string,
        currentSummary: view["currentSummary"].string,
        everyday: options(view, "everyday"),
        folded: options(view, "folded"),
        all: options(view, "modes"),
        hidden: view["hidden"].arrayOrNil?.map(\.string) ?? [],
        hiddenSetting: hiddenSetting.isBlank ? nil : hiddenSetting,
        unknownModeNotice: view["unknownModeNotice"].string
    )
}

struct ModeAck: Equatable {
    let serial: Int64
    let accepted: Bool
    let wording: String
}

func modeAck(_ view: JSON?) -> ModeAck? {
    guard let ack = view?["modeAck"], ack.object != nil else { return nil }
    return ModeAck(serial: ack["serial"].int64 ?? 0, accepted: ack["accepted"].bool, wording: ack["wording"].string)
}

let MODE_REPLY_MS: Int64 = 3000
let MODE_REJECTION_MS: Int64 = 2500

enum ModeOutcome: Equatable {
    case Pending
    case Settled
    case Rejected(String)
}

func modeOutcome(_ mode: String, _ before: ModeAck?, _ now: ModeAck?, _ reached: Bool, _ elapsedMs: Int64) -> ModeOutcome {
    if reached { return .Settled }
    if let now, now.serial != before?.serial, !now.accepted { return .Rejected("\(mode) \(now.wording)") }
    if elapsedMs >= MODE_REPLY_MS { return .Rejected("\(mode): no reply") }
    return .Pending
}

func hiddenModesAfter(_ hidden: [String], _ mode: String, _ hide: Bool) -> String {
    (hidden.filter { $0 != mode } + (hide ? [mode] : [])).joined(separator: ",")
}

func startsSection(_ shown: [FlightModeOption], _ index: Int) -> Bool {
    index > 0 && shown[index - 1].section != shown[index].section
}

let HIDDEN_MODE_ALPHA = 0.55

func modeHeading(_ modes: FlightModesView?) -> String? {
    guard let modes, !modes.currentSummary.isBlank, !modes.current.isBlank else { return nil }
    return "\(modes.current) — \(modes.currentSummary)"
}
