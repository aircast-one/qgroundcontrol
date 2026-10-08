import Foundation

let PWM_MIN = 1000
let PWM_CENTER = 1500
let PWM_MAX = 2000

enum RcControlType: CaseIterable, Hashable { case Slider, Button, Switch3, Momentary }

struct RcControl: Equatable {
    let label: String
    let channel: Int
    let type: RcControlType
}

private func typeOf(_ raw: String) -> RcControlType? {
    switch raw {
    case "slider": .Slider
    case "button": .Button
    case "switch3": .Switch3
    case "momentary": .Momentary
    default: nil
    }
}

func parseRcControls(_ json: String?) -> [RcControl] {
    guard let array = JSON.parse(json ?? "").arrayOrNil else { return [] }
    return array.filter { $0.object != nil }.compactMap { entry in
        let channel = entry["channel"].int(0)
        guard channel > 0, let type = typeOf(entry["type"].string) else { return nil }
        return RcControl(label: entry["label"].string.ifBlank("CH\(channel)"), channel: channel, type: type)
    }
}

func switch3Pwms() -> [Int] { [PWM_MIN, PWM_CENTER, PWM_MAX] }

let RC_CHANNEL_MIN = 1
let RC_CHANNEL_MAX = 18

func typeName(_ type: RcControlType) -> String {
    switch type {
    case .Slider: "slider"
    case .Button: "button"
    case .Switch3: "switch3"
    case .Momentary: "momentary"
    }
}

func typeLabel(_ type: RcControlType) -> String {
    switch type {
    case .Slider: "Slider"
    case .Button: "Button"
    case .Switch3: "Three-way switch"
    case .Momentary: "Momentary"
    }
}

private func entries(_ json: String?) -> [JSON] {
    (JSON.parse(json ?? "").arrayOrNil ?? []).map { $0.object != nil ? $0 : .object([:]) }
}

private func encode(_ entries: [JSON]) -> String { JSON.array(entries).text }

private func patched(_ entry: JSON, _ label: String, _ channel: Int, _ type: RcControlType) -> JSON {
    .object((entry.object ?? [:]).merging(["label": .string(label), "channel": .number(Double(channel)), "type": .string(typeName(type))]) { $1 })
}

func rcControlsAdded(_ json: String?, _ label: String, _ channel: Int, _ type: RcControlType) -> String {
    encode(entries(json) + [patched(.object([:]), label, channel, type)])
}

func rcControlsRemoved(_ json: String?, _ index: Int) -> String {
    encode(entries(json).enumerated().filter { $0.offset != index }.map(\.element))
}

func rcControlsPatched(_ json: String?, _ index: Int, _ label: String, _ channel: Int, _ type: RcControlType) -> String {
    encode(entries(json).enumerated().map { at, entry in at == index ? patched(entry, label, channel, type) : entry })
}

func channelOwner(_ json: String?, _ channel: Int, _ ignoring: Int, _ reserved: [Int: String]) -> String? {
    reserved[channel] ?? entries(json).enumerated()
        .first { at, entry in at != ignoring && entry["channel"].int(0) == channel }
        .map { at, entry in entry["label"].string.ifBlank("control \(at + 1)") }
}

func firstFreeChannel(_ json: String?, _ reserved: [Int: String]) -> Int {
    (RC_CHANNEL_MIN...RC_CHANNEL_MAX).first { channelOwner(json, $0, -1, reserved) == nil } ?? RC_CHANNEL_MIN
}

func channelUsable(_ channel: Int) -> Bool { (RC_CHANNEL_MIN...RC_CHANNEL_MAX).contains(channel) }

let RC_SEND_INTERVAL_MS: Int64 = 100

func rcSendDue(_ nowMs: Int64, _ lastSentMs: Int64, _ finished: Bool) -> Bool {
    finished || nowMs - lastSentMs >= RC_SEND_INTERVAL_MS
}
