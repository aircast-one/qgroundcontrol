import Foundation

struct RadioChannel: Identifiable, Equatable {
    let index: Int
    let label: String
    let value: Int
    let valueText: String
    let fraction: Double
    let live: Bool

    var id: Int { index }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let index = (json["index"] as? NSNumber)?.intValue else { return nil }
        self.index = index
        label = (json["label"] as? String) ?? ""
        value = (json["value"] as? NSNumber)?.intValue ?? 0
        valueText = (json["valueText"] as? String) ?? ""
        fraction = (json["fraction"] as? NSNumber)?.doubleValue ?? 0
        live = (json["live"] as? NSNumber)?.boolValue ?? false
    }
}

struct RadioStick: Identifiable, Equatable {
    let key: String
    let title: String
    let mapped: Bool
    let value: Int
    let valueText: String
    let fraction: Double
    let reversed: Bool

    var id: String { key }

    static let reversedNote = "Reversed"
    var reversedText: String { reversed ? RadioStick.reversedNote : "" }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let key = json["key"] as? String else { return nil }
        self.key = key
        title = (json["title"] as? String) ?? ""
        mapped = (json["mapped"] as? NSNumber)?.boolValue ?? false
        value = (json["value"] as? NSNumber)?.intValue ?? 0
        valueText = (json["valueText"] as? String) ?? ""
        fraction = (json["fraction"] as? NSNumber)?.doubleValue ?? 0
        reversed = (json["reversed"] as? NSNumber)?.boolValue ?? false
    }
}

struct RadioState: Equatable {
    let connected: Bool
    let channelCount: Int
    let summary: String
    let notReady: String
    let calibrating: Bool
    let statusText: String
    let nextText: String
    let nextEnabled: Bool
    let cancelEnabled: Bool
    let skipEnabled: Bool
    let transmitterMode: Int
    let channels: [RadioChannel]
    let sticks: [RadioStick]

    static let disconnected = RadioState()

    static let defaultTransmitterMode = 2

    private init() {
        connected = false
        channelCount = 0
        summary = ""
        notReady = ""
        calibrating = false
        statusText = ""
        nextText = ""
        nextEnabled = false
        cancelEnabled = false
        skipEnabled = false
        transmitterMode = RadioState.defaultTransmitterMode
        channels = []
        sticks = []
    }

    init(_ json: [String: Any]) {
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        connected = flag("connected")
        channelCount = (json["channelCount"] as? NSNumber)?.intValue ?? 0
        summary = text("summary")
        notReady = ((json["notReady"] as? [String: Any])?["message"] as? String) ?? ""
        calibrating = flag("calibrating")
        statusText = text("statusText")
        nextText = text("nextText")
        nextEnabled = flag("nextEnabled")
        cancelEnabled = flag("cancelEnabled")
        skipEnabled = flag("skipEnabled")
        transmitterMode = (json["transmitterMode"] as? NSNumber)?.intValue
            ?? RadioState.defaultTransmitterMode
        channels = ((json["channels"] as? [Any]) ?? []).compactMap(RadioChannel.init)
        sticks = ((json["sticks"] as? [Any]) ?? []).compactMap(RadioStick.init)
    }

    static let defaultNextText = "Calibrate"

    var actionTitle: String { nextText.isEmpty ? RadioState.defaultNextText : nextText }

    var level: FlyTelemetry.Level {
        guard connected, channelCount > 0 else { return .unknown }
        return liveChannels.isEmpty ? .warning : .good
    }

    var symbol: String {
        level == .good
            ? "antenna.radiowaves.left.and.right"
            : "antenna.radiowaves.left.and.right.slash"
    }

    var liveChannels: [RadioChannel] { channels.filter(\.live) }

    var channelRows: [RadioChannel] { channels }
}
