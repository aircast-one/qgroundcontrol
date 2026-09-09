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
    let shortfall: String
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

    // QGC's RadioComponentController starts at mode 2, so a reply that carries no mode at all
    // must not be read as mode 0, which is not a mode any transmitter has.
    static let defaultTransmitterMode = 2

    private init() {
        connected = false
        channelCount = 0
        summary = ""
        shortfall = ""
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
        shortfall = text("shortfall")
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

    var liveChannels: [RadioChannel] { channels.filter(\.live) }
}
