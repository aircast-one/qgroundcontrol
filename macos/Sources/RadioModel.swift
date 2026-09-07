import Foundation

struct RadioChannel: Identifiable, Equatable {
    let index: Int
    let value: Int

    var id: Int { index }

    var label: String { "\(index + 1)" }

    var valueText: String { value > 0 ? "\(value)" : "\u{2014}" }

    var fraction: Double { RadioState.fraction(value) }

    var live: Bool { value > 0 }
}

struct RadioStick: Identifiable, Equatable {
    let key: String
    let title: String
    let mapped: Bool
    let value: Int
    let reversed: Bool

    var id: String { key }

    var valueText: String { mapped ? (value > 0 ? "\(value)" : "\u{2014}") : "Not mapped" }

    var fraction: Double { RadioState.fraction(value) }
}

struct RadioState: Equatable {
    var connected = false
    var channelCount = 0
    var minimumChannels = 0
    var channels: [RadioChannel] = []
    var sticks: [RadioStick] = []
    var statusText = ""
    var nextText = ""
    var nextEnabled = false
    var cancelEnabled = false
    var skipEnabled = false
    var transmitterMode = 2

    static let disconnected = RadioState()

    static let lowPwm = 1000.0
    static let highPwm = 2000.0

    static func fraction(_ value: Int) -> Double {
        guard value > 0 else { return 0 }
        let clamped = min(max(Double(value), lowPwm), highPwm)
        return (clamped - lowPwm) / (highPwm - lowPwm)
    }

    var liveChannels: [RadioChannel] { channels.filter(\.live) }

    var enoughChannels: Bool { channelCount >= minimumChannels }

    var calibrating: Bool { cancelEnabled }

    var summary: String {
        guard connected else { return "No vehicle is connected." }
        guard channelCount > 0 else { return "No transmitter is being heard." }
        let live = liveChannels.count
        return "\(channelCount) channel\(channelCount == 1 ? "" : "s") reported, \(live) carrying a signal."
    }

    var shortfall: String {
        guard connected, channelCount > 0, !enoughChannels else { return "" }
        return "At least \(minimumChannels) channels are needed to fly; the transmitter reports \(channelCount)."
    }
}

enum Radio {
    static let stickOrder: [(key: String, title: String)] = [
        ("roll", "Roll"),
        ("pitch", "Pitch"),
        ("yaw", "Yaw"),
        ("throttle", "Throttle"),
    ]

    static func property(_ key: String, _ suffix: String) -> String {
        "\(key)Channel\(suffix)"
    }

    static func sticks(from json: [String: Any]) -> [RadioStick] {
        stickOrder.map { entry in
            RadioStick(
                key: entry.key,
                title: entry.title,
                mapped: (json[property(entry.key, "Mapped")] as? NSNumber)?.boolValue ?? false,
                value: (json[property(entry.key, "RCValue")] as? NSNumber)?.intValue ?? 0,
                reversed: ((json[property(entry.key, "Reversed")] as? NSNumber)?.intValue ?? 0) != 0)
        }
    }

    static func read(_ json: [String: Any]) -> RadioState {
        guard json["kind"] as? String == "object" else { return .disconnected }

        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func number(_ name: String) -> Int { (json[name] as? NSNumber)?.intValue ?? 0 }

        var state = RadioState()
        state.connected = true
        state.channelCount = number("channelCount")
        state.minimumChannels = number("minChannelCount")
        state.channels = ((json["rcValues"] as? [Any]) ?? [])
            .enumerated()
            .map { RadioChannel(index: $0.offset, value: ($0.element as? NSNumber)?.intValue ?? 0) }
        state.sticks = sticks(from: json)
        state.statusText = (json["statusText"] as? String) ?? ""
        state.nextText = (json["nextText"] as? String) ?? ""
        state.nextEnabled = flag("nextEnabled")
        state.cancelEnabled = flag("cancelEnabled")
        state.skipEnabled = flag("skipEnabled")
        state.transmitterMode = number("transmitterMode")
        return state
    }
}
