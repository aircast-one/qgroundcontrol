import SwiftUI

let PACKET_RADIO_VIEW = "view.packetRadio"
private let WAITING = "waiting"

struct PacketRadioStatus: Equatable {
    let statusText: String
    let linkActive: Bool
    let haveSignal: Bool
    let signal: String
    let noise: String
    let linkScore: String
    let packetLoss: String
    let videoPackets: String
    let adapters: [String]
}

private func joined(_ values: JSON) -> String { values.array.map(\.string).joined(separator: "  |  ") }

func packetRadioStatus(_ view: JSON?) -> PacketRadioStatus? {
    guard let view, view["class"].string == "PacketRadio" else { return nil }
    let signal = view["haveSignal"].bool
    return PacketRadioStatus(
        statusText: view["statusText"].string.ifBlank("Unavailable"),
        linkActive: view["linkActive"].bool,
        haveSignal: signal,
        signal: signal ? joined(view["antennaRssiRaw"]) : WAITING,
        noise: signal ? joined(view["antennaSnr"]) + " dB" : WAITING,
        linkScore: signal && !view["linkScore"].isNull ? String(view["linkScore"].int(0)) : WAITING,
        packetLoss: view["packetLoss"].isNull ? "" : String(view["packetLoss"].int(0)),
        videoPackets: view["videoPackets"].isNull ? "" : String(view["videoPackets"].int64 ?? 0),
        adapters: view["adapters"].strings
    )
}
