import Foundation

struct PacketRadioReading: Equatable {
    let rssi: Double?
    let snr: Double?
    let score: Double?
}

struct PacketRadio: Equatable {
    let status: String
    let statusText: String
    let running: Bool
    let starting: Bool
    let linkActive: Bool
    let adapter: String
    let adapters: [String]
    let unsupportedAdapters: [String]
    let startError: String
    let readings: [PacketRadioReading]
    let haveSignal: [Bool]
    let linkScore: Double?
    let linkScoreMin: Double
    let linkScoreMax: Double
    let packetLoss: Double?
    let stale: Bool
    let rssiUnit: String
    let snrUnit: String
    let packetLossUnit: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        func number(_ name: String) -> Double? { (json[name] as? NSNumber)?.doubleValue }
        func list(_ name: String) -> [Double?] {
            (json[name] as? [Any])?.map { ($0 as? NSNumber)?.doubleValue } ?? []
        }

        status = text("status")
        statusText = text("statusText")
        running = flag("running")
        starting = flag("starting")
        linkActive = flag("linkActive")
        adapter = text("adapter")
        adapters = (json["adapters"] as? [Any])?.compactMap { $0 as? String } ?? []
        unsupportedAdapters = (json["unsupportedAdapters"] as? [Any])?
            .compactMap { $0 as? String } ?? []
        startError = text("startError")

        let rssi = list("antennaRssi")
        let snr = list("antennaSnr")
        let score = list("antennaScore")
        let antennas = max(rssi.count, max(snr.count, score.count))
        readings = (0..<antennas).map {
            PacketRadioReading(rssi: $0 < rssi.count ? rssi[$0] : nil,
                               snr: $0 < snr.count ? snr[$0] : nil,
                               score: $0 < score.count ? score[$0] : nil)
        }
        haveSignal = (json["haveSignal"] as? [Any])?
            .map { ($0 as? NSNumber)?.boolValue ?? false } ?? []

        linkScore = number("linkScore")
        linkScoreMin = number("linkScoreMin") ?? 0
        linkScoreMax = number("linkScoreMax") ?? 0
        packetLoss = number("packetLoss")
        stale = flag("stale")
        rssiUnit = text("rssiUnit")
        snrUnit = text("snrUnit")
        packetLossUnit = text("packetLossUnit")
    }

    var linkScoreFraction: Double? {
        guard let linkScore, linkScoreMax > linkScoreMin else { return nil }
        return (linkScore - linkScoreMin) / (linkScoreMax - linkScoreMin)
    }

    func rssiText(_ antenna: Int) -> String { Self.show(reading(antenna)?.rssi, rssiUnit) }

    func snrText(_ antenna: Int) -> String { Self.show(reading(antenna)?.snr, snrUnit) }

    var packetLossText: String { Self.show(packetLoss, packetLossUnit) }

    func reading(_ antenna: Int) -> PacketRadioReading? {
        antenna >= 0 && antenna < readings.count ? readings[antenna] : nil
    }

    // A reading the radio has not taken is NOT zero: 0 dBm is a stronger signal than any real
    // antenna reports, and 0% loss is a perfect link. An absent answer and an answer of "no
    // problem" must not render the same.
    static func show(_ value: Double?, _ unit: String) -> String {
        guard let value, value.isFinite else { return "" }
        let rounded = (value * 10).rounded() / 10
        let digits = rounded == rounded.rounded() ? 0 : 1
        let printed = Measure.settled(String(format: "%.\(digits)f", rounded))
        return unit.isEmpty ? printed : printed + " " + unit
    }
}
