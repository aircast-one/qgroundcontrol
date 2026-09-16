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
    let haveSignal: Bool?
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
        haveSignal = (json["haveSignal"] as? NSNumber)?.boolValue

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

    // A RECEIVER WITH NO STATISTICS YET IS NOT A RECEIVER WITH NO SIGNAL -- packetradio.rs:895
    // asserts exactly that, and haveSignal is null for the first and false for the second. The
    // head collapsed them into one sentence, so a radio that was running and hearing nothing read
    // as one that had not reported yet: the operator waits instead of checking the antenna.
    static let noReadings = "No signal readings yet."
    static let hearingNothing = "The receiver is running and hearing nothing."

    var emptyText: String {
        haveSignal == false ? PacketRadio.hearingNothing : PacketRadio.noReadings
    }

    // readings.isEmpty was the wrong question and it made the sentence above unreachable.
    // antennaSnr and antennaScore are plain [i32; 2] the core fills whenever a Reading exists at
    // all (packetradio.rs:513-514 map the whole Reading), so they are never absent and the list
    // is non-empty for a radio hearing NOTHING. Only antennaRssi carries per-antenna presence,
    // through rssi_dbm()'s Option. CONSTRUCTED through the parameterised view --
    // view.packetRadio(receiving,wfb0,0/0,12/6,1800/2500,0) gives antennaRssi [null, null],
    // antennaSnr [12, 6], haveSignal false -- and the panel drew two rows of bare SNR numbers
    // instead of the sentence added for exactly that state.
    var listsAntennas: Bool { haveSignal == true }

    // An antenna the receiver is not hearing keeps its slot and says so. It used to print its SNR
    // alone, on the stated reason that "the two readings are independently absent" -- which the
    // producer contradicts: snr is an i32 and is never absent while rssi can be. So the row that
    // most needs to say THIS ANTENNA IS HEARING NOTHING was the one that simply looked shorter
    // than its neighbour, and the operator had to infer which figure had gone from its unit.
    // CONSTRUCTED: ...(receiving,wfb0,0/26,12/6,1800/2500,0) gives antennaRssi [null, -84].
    func antennaText(_ antenna: Int) -> String {
        let level = rssiText(antenna)
        return (level.isEmpty ? Measure.unreported : level) + "   " + snrText(antenna)
    }

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

enum AdapterChoice {
    static let automatic = "Automatic"

    static func missing(_ name: String) -> String { "\(name) \u{2014} not connected" }

    // A configured adapter that is not in the list gets a row of its own. It used to collapse to
    // index 0, which is AUTOMATIC -- so unplugging the adapter made the picker show a choice the
    // operator never made, and any interaction landing on 0 wrote "" and erased their preference.
    // Two different situations, one index: an empty deviceName really is Automatic, and a named
    // adapter that is absent is a preference being kept for a device that is not here.
    static func options(_ adapters: [String], configured: String = "") -> [String] {
        let listed = [automatic] + adapters
        guard !configured.isEmpty, !adapters.contains(configured) else { return listed }
        return listed + [missing(configured)]
    }

    // The picker shows the CONFIGURED preference, not the adapter currently open: with
    // Automatic selected those differ, and showing the open one would make the box read as a
    // choice nobody made. An empty deviceName is Automatic.
    static func selected(deviceName: String, adapters: [String]) -> Int {
        guard !deviceName.isEmpty else { return 0 }
        guard let found = adapters.firstIndex(of: deviceName) else { return adapters.count + 1 }
        return found + 1
    }

    // The absent row writes the SAME name back rather than "", so selecting what is already
    // selected cannot be the thing that clears it.
    static func chosen(index: Int, adapters: [String], configured: String = "") -> String {
        if index >= 1 && index - 1 < adapters.count { return adapters[index - 1] }
        if index == adapters.count + 1 { return configured }
        return ""
    }
}
