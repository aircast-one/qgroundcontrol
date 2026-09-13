import SwiftUI

final class PacketRadioStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "packetRadio"

    @Published private(set) var radio: PacketRadio?
    @Published private(set) var deviceName = ""
    @Published var writeFailure: String?

    private var timer: Timer?

    func startPolling() {
        guard timer == nil else { return }
        refresh()
        let timer = Timer(timeInterval: 1.0, repeats: true) { [weak self] _ in self?.refresh() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stopPolling() {
        timer?.invalidate()
        timer = nil
    }

    func refresh() {
        let read = PacketRadio(Bridge.group("view.packetRadio"))
        if read != radio { radio = read }
        let configured = (Bridge.group(Self.deviceNamePath)["value"] as? String) ?? ""
        if configured != deviceName { deviceName = configured }
    }

    static let deviceNamePath = "settings.packetRadioSettings.deviceName"

    func chooseAdapter(_ index: Int) {
        let name = AdapterChoice.chosen(index: index, adapters: radio?.adapters ?? [])
        guard name != deviceName else { return }
        if write(Self.deviceNamePath, name, "the Wi-Fi adapter") { deviceName = name }
    }

    func probeState() -> [String: Any] {
        guard let radio else { return ["reported": false] }
        return ["reported": true,
                "status": radio.status,
                "statusText": radio.statusText,
                "running": radio.running,
                "linkActive": radio.linkActive,
                "adapter": radio.adapter,
                "adapters": radio.adapters,
                "unsupportedAdapters": radio.unsupportedAdapters,
                "startError": radio.startError,
                "stale": radio.stale,
                "deviceName": deviceName,
                "adapterChoice": AdapterChoice.selected(deviceName: deviceName,
                                                        adapters: radio.adapters),
                "antennas": radio.readings.indices.map {
                    ["rssi": radio.rssiText($0), "snr": radio.snrText($0)]
                },
                "packetLoss": radio.packetLossText,
                "linkScore": radio.linkScoreFraction ?? -1]
    }
}

struct PacketRadioSection: View {
    @ObservedObject var store: PacketRadioStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Radio").font(.headline).padding(.bottom, 8)
            if let radio = store.radio {
                reported(radio)
            } else {
                Text("No packet radio has been reported on this machine.")
                    .foregroundColor(.secondary)
                    .padding(.vertical, 8)
            }
        }
        .onAppear(perform: store.startPolling)
        .onDisappear(perform: store.stopPolling)
    }

    @ViewBuilder
    private func reported(_ radio: PacketRadio) -> some View {
        if !radio.statusText.isEmpty {
            Text(radio.statusText)
                .foregroundColor(radio.startError.isEmpty ? .primary : .red)
                .padding(.bottom, 6)
        }

        Picker("Wi-Fi adapter", selection: Binding(
            get: { AdapterChoice.selected(deviceName: store.deviceName,
                                          adapters: radio.adapters) },
            set: { store.chooseAdapter($0) })) {
            ForEach(Array(AdapterChoice.options(radio.adapters).enumerated()), id: \.offset) {
                index, label in Text(label).tag(index)
            }
        }
        .padding(.bottom, 6)

        if !radio.adapter.isEmpty {
            row("In use", radio.adapter)
        }

        if !radio.unsupportedAdapters.isEmpty {
            row("Not supported", radio.unsupportedAdapters.joined(separator: ", "))
        }

        if radio.readings.isEmpty {
            Text("No signal readings yet.")
                .foregroundColor(.secondary)
                .padding(.vertical, 6)
        } else {
            Divider()
            ForEach(radio.readings.indices, id: \.self) { antenna in
                row("Antenna \(antenna + 1)", reading(radio, antenna))
            }
            if !radio.packetLossText.isEmpty {
                row("Packet loss", radio.packetLossText)
            }
            if radio.stale {
                Text("These readings have stopped updating.")
                    .foregroundColor(.secondary)
                    .padding(.top, 4)
            }
        }
    }

    // An antenna the radio has taken no reading from shows nothing rather than a dash beside a
    // number, because the two readings are independently absent.
    private func reading(_ radio: PacketRadio, _ antenna: Int) -> String {
        [radio.rssiText(antenna), radio.snrText(antenna)]
            .filter { !$0.isEmpty }
            .joined(separator: "   ")
    }

    private func row(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value.isEmpty ? "Not reported" : value)
                .foregroundColor(value.isEmpty ? .secondary : .primary)
        }
        .padding(.vertical, 4)
    }
}
