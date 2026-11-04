#if os(macOS)
import SwiftUI

// The packet radio (wfb-ng over a USB Wi-Fi adapter) needs the WFB driver a Mac can link and an iPhone cannot, so this
// section exists on the native macOS app only (see ANDROID_PARITY.md).

let PACKET_RADIO_VIEW = "view.packetRadio"
let PACKET_RADIO_REFRESH = "packetRadio.refreshAdapters"
private let PACKET_RADIO_POLL_MS = 1000
private let PACKET_RADIO_DEVICE = "settings.packetRadioSettings.deviceName.rawValue"
private let AUTOMATIC_ADAPTER = "Automatic"
private let PACKET_RADIO_WAITING = "waiting"

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

private func packetRadioJoined(_ values: JSON) -> String {
    values.array.map(\.string).joined(separator: "  |  ")
}

func packetRadioStatus(_ view: JSON?) -> PacketRadioStatus? {
    guard let view, view["class"].string == "PacketRadio" else { return nil }
    let signal = view["haveSignal"].bool
    return PacketRadioStatus(
        statusText: view["statusText"].string.ifBlank("Unavailable"),
        linkActive: view["linkActive"].bool,
        haveSignal: signal,
        signal: signal ? packetRadioJoined(view["antennaRssiRaw"]) : PACKET_RADIO_WAITING,
        noise: signal ? packetRadioJoined(view["antennaSnr"]) + " dB" : PACKET_RADIO_WAITING,
        linkScore: signal ? (view["linkScore"].int.map { String($0) } ?? PACKET_RADIO_WAITING) : PACKET_RADIO_WAITING,
        packetLoss: view["packetLoss"].int.map { String($0) } ?? "",
        videoPackets: view["videoPackets"].int64.map { String($0) } ?? "",
        adapters: view["adapters"].strings
    )
}

// The adapter menu: Automatic first, then every adapter the radio lists. Picking Automatic writes "".
func packetRadioAdapterChoices(_ status: PacketRadioStatus?) -> [String] {
    [AUTOMATIC_ADAPTER] + (status?.adapters ?? [])
}

func packetRadioAdapterWrite(_ index: Int, _ name: String) -> String { index == 0 ? "" : name }

struct PacketRadioSection: View {
    let onWrite: () -> Void
    @State private var read: PacketRadioStatus?
    @State private var device = ""
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            native
        } else {
            handDrawn
        }
    }

    // Inside a NativeForm: Status (whose row carries the poll), Link quality while the link is up, and Radio with
    // the adapter as a menu row.
    @ViewBuilder private var native: some View {
        Section {
            LabeledContent("Status", value: read?.statusText ?? "Unavailable")
                .task { await poll() }
        }
        if let status = read, status.linkActive {
            Section {
                LabeledContent("Signal per antenna (0-126)", value: status.signal)
                LabeledContent("Noise margin per antenna", value: status.noise)
                LabeledContent("Link score (1000-2000)", value: status.linkScore)
                LabeledContent("Packets lost (last second)", value: status.packetLoss)
                LabeledContent("Video packets", value: status.videoPackets)
            } header: {
                SectionHeader(text: "Link quality")
            }
        }
        Section {
            let choices = packetRadioAdapterChoices(read)
            ChoiceField(label: "Wi-Fi adapter", value: device.ifBlank(AUTOMATIC_ADAPTER), options: choices) { index in
                if choices.indices.contains(index) { choose(packetRadioAdapterWrite(index, choices[index])) }
            }
        } header: {
            SectionHeader(text: "Radio")
        }
    }

    @MainActor private func poll() async {
        _ = await offMain { Qgc.invoke(PACKET_RADIO_REFRESH) }
        while !Task.isCancelled {
            let next = await offMain { packetRadioStatus(Qgc.get(PACKET_RADIO_VIEW)) }
            if next != read { read = next }
            let configured = await offMain { Qgc.get(PACKET_RADIO_DEVICE)["value"].string }
            if configured != device { device = configured }
            try? await Task.sleep(for: .milliseconds(PACKET_RADIO_POLL_MS))
        }
    }

    private var handDrawn: some View {
        VStack(alignment: .leading, spacing: 6) {
            PacketRadioLine(label: "Status", value: read?.statusText ?? "Unavailable")
            if let status = read, status.linkActive {
                Text("Link quality").font(.titleSmall)
                PacketRadioLine(label: "Signal per antenna (0-126)", value: status.signal)
                PacketRadioLine(label: "Noise margin per antenna", value: status.noise)
                PacketRadioLine(label: "Link score (1000-2000)", value: status.linkScore)
                PacketRadioLine(label: "Packets lost (last second)", value: status.packetLoss)
                PacketRadioLine(label: "Video packets", value: status.videoPackets)
            }
            Text("Radio").font(.titleSmall)
            HStack {
                Text("Wi-Fi adapter").frame(maxWidth: .infinity, alignment: .leading)
                Menu {
                    ForEach(Array(packetRadioAdapterChoices(read).enumerated()), id: \.offset) { index, name in
                        Button(name) { choose(packetRadioAdapterWrite(index, name)) }
                    }
                } label: {
                    Text(device.ifBlank(AUTOMATIC_ADAPTER))
                }
                .fixedSize()
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
        .task { await poll() }
    }

    @MainActor private func choose(_ name: String) {
        Task {
            _ = await offMain { Qgc.set(PACKET_RADIO_DEVICE, name) }
            device = name
            onWrite()
        }
    }
}

private struct PacketRadioLine: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label).font(.bodySmall).frame(maxWidth: .infinity, alignment: .leading)
            Text(value).font(.bodySmall)
        }
    }
}
#endif
