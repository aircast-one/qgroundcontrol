import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct FirmwarePort: Equatable, Identifiable {
    let port: String
    let description: String
    let bootloader: Bool

    var id: String { port }
    var title: String { description.isEmpty ? port : "\(port) — \(description)" }
}

struct FirmwareJob: Equatable {
    var phase = "idle"
    var busy = false
    var progress = 0.0
    var messages: [String] = []
    var error = ""
    var ports: [FirmwarePort] = []

    static let fileExtensions = ["px4", "apj", "bin"]

    static func read(_ view: [String: Any], ports listed: [String: Any]) -> FirmwareJob? {
        guard view["kind"] as? String == "object" else { return nil }
        let ports = (listed["ports"] as? [[String: Any]] ?? []).compactMap { entry -> FirmwarePort? in
            guard let port = entry["port"] as? String else { return nil }
            return FirmwarePort(port: port,
                                description: entry["description"] as? String ?? "",
                                bootloader: (entry["bootloader"] as? NSNumber)?.boolValue ?? false)
        }
        return FirmwareJob(phase: view["phase"] as? String ?? "idle",
                           busy: (view["busy"] as? NSNumber)?.boolValue ?? false,
                           progress: (view["progress"] as? NSNumber)?.doubleValue ?? 0,
                           messages: view["messages"] as? [String] ?? [],
                           error: view["error"] as? String ?? "",
                           ports: ports)
    }

    var phaseText: String {
        switch phase {
        case "connecting": return "Waiting for the bootloader"
        case "erasing": return "Erasing"
        case "programming": return "Programming"
        case "verifying": return "Verifying"
        case "complete": return "Upgrade complete"
        case "failed": return "Upgrade failed"
        default: return ""
        }
    }
}

final class FirmwareStore: ObservableObject, Probeable {
    static let probeID = "firmware"

    @Published private(set) var job = FirmwareJob()
    @Published var port = ""
    @Published private(set) var file = ""
    @Published private(set) var refusal = ""

    private var poll: Timer?

    func startWatching() {
        guard poll == nil else { return }
        reload()
        poll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in self?.reload() }
    }

    func stopWatching() {
        poll?.invalidate()
        poll = nil
    }

    func reload() {
        guard let read = FirmwareJob.read(Bridge.group("view.firmwareUpgrade"), ports: Bridge.group("view.firmwarePorts")) else { return }
        if read != job { job = read }
        if port.isEmpty, let first = read.ports.first(where: \.bootloader) {
            port = first.port
        }
    }

    func chooseFile() {
        let panel = NSOpenPanel()
        panel.title = "Select the firmware file"
        panel.allowedContentTypes = FirmwareJob.fileExtensions.compactMap { UTType(filenameExtension: $0) }
        panel.allowsOtherFileTypes = true
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        file = url.path
    }

    var canFlash: Bool { !job.busy && !port.isEmpty && !file.isEmpty }

    func flash() {
        guard canFlash else { return }
        let answer = Bridge.invoke("firmware.flash", [port, file])
        refusal = (answer["ok"] as? NSNumber)?.boolValue == true ? "" : (answer["reason"] as? String ?? "The upgrade did not start.")
        reload()
    }

    func probeState() -> [String: Any] {
        ["phase": job.phase, "busy": job.busy, "progress": job.progress, "port": port, "file": file,
         "ports": job.ports.map(\.port), "messages": job.messages, "error": job.error, "refusal": refusal]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "select":
            port = args["port"] ?? port
            file = args["file"] ?? file
            return ["ok": true]
        case "flash":
            flash()
            return ["ok": refusal.isEmpty, "refusal": refusal]
        default:
            return ["ok": false, "error": "firmware has no action \(action)"]
        }
    }
}

struct FirmwareView: View {
    @ObservedObject var store: FirmwareStore

    var body: some View {
        SetupPageBody(title: "Firmware",
                      note: "Plug in your device via USB, choose its port and a firmware file, then press Flash. A board running its firmware is asked to be unplugged and plugged back in so its bootloader starts.") {
            GroupCard {
                GroupRow(title: "Port", showSeparator: false, leading: { Tile(symbol: "cable.connector", colour: .blue) }) {
                    Picker("", selection: $store.port) {
                        if store.job.ports.isEmpty && store.port.isEmpty { Text("No serial ports").tag("") }
                        if !store.port.isEmpty && !store.job.ports.contains(where: { $0.port == store.port }) {
                            Text("\(store.port) — not plugged in").tag(store.port)
                        }
                        ForEach(store.job.ports) { Text($0.title).tag($0.port) }
                    }
                    .labelsHidden()
                    .frame(maxWidth: 320)
                    .disabled(store.job.busy)
                }
                GroupRow(title: "Firmware file",
                         description: store.file.isEmpty ? "A .px4, .apj or .bin image" : store.file,
                         leading: { Tile(symbol: "doc.zipper", colour: .indigo) }) {
                    Button("Choose\u{2026}", action: store.chooseFile).disabled(store.job.busy)
                }
            }

            HStack(spacing: Overlay.step) {
                Button("Flash", action: store.flash).disabled(!store.canFlash)
                if store.job.busy {
                    ProgressView(value: store.job.progress).frame(maxWidth: 240)
                }
                Text(store.job.phaseText).foregroundStyle(.secondary)
                Spacer()
            }

            if !store.refusal.isEmpty {
                Text(store.refusal).foregroundStyle(.red)
            }
            if !store.job.error.isEmpty {
                Text(store.job.error).foregroundStyle(.red).textSelection(.enabled)
            }
            if !store.job.messages.isEmpty {
                GroupCard {
                    Text(store.job.messages.joined(separator: "\n"))
                        .font(.system(.body, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(Overlay.step)
                }
            }
        }
        .onAppear(perform: store.startWatching)
        .onDisappear(perform: store.stopWatching)
    }
}
