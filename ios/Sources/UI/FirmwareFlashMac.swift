#if os(macOS)
import SwiftUI
import UniformTypeIdentifiers

// USB firmware flashing, as Android's FirmwareScreen does it. Only the Mac has the serial ports the core flashes
// through; the iOS FirmwareScreen in FirmwareScreen.swift lists the releases and says why it cannot flash.

let FIRMWARE_PORTS_VIEW = "view.firmwarePorts"
let FIRMWARE_FLASH = "firmware.flash"
let FIRMWARE_CANCEL = "firmware.cancel"
let FIRMWARE_CHOOSE = "firmware.choose"
let FIRMWARE_UPGRADE_SETTINGS = "settings.firmwareUpgradeSettings"
let APM_CHIBIOS = "apmChibiOS"
let FIRMWARE_POLL_MS = 500
let FIRMWARE_EXTENSIONS = ["px4", "apj", "bin", "ihx"]
private let FIRMWARE_STAGE_PREFIX = "firmware-"

struct FirmwarePort: Equatable, Identifiable {
    let port: String
    let description: String
    let bootloader: Bool
    var boardType: String = ""

    var id: String { port }
}

let MULTIPLE_DEVICES = "Multiple devices detected. Make sure to select the correct one from the list."

func preselectedPort(_ ports: [FirmwarePort], _ current: String) -> String? {
    if let kept = ports.first(where: { $0.port == current && !$0.boardType.isBlank }) { return kept.port }
    return (ports.first { $0.boardType == "Pixhawk" } ?? ports.first { $0.boardType == "SiK Radio" })?.port
}

struct FirmwareBuildChoice: Equatable, Hashable {
    let name: String
    let url: String
}

struct FirmwareJob: Equatable {
    var phase: String
    var busy: Bool
    var cancellable: Bool
    var progress: Double
    var messages: [String]
    var error: String
    var choices: [FirmwareBuildChoice] = []
    var bestChoice: Int = -1
    var updateAvailable: String = ""
    var px4StableVersion: String = ""
    var px4BetaVersion: String = ""
}

func firmwarePorts(_ view: JSON?) -> [FirmwarePort] {
    (view?["ports"].objects ?? []).compactMap { entry in
        let port = entry["port"].string
        guard !port.isBlank else { return nil }
        return FirmwarePort(port: port, description: entry["description"].string, bootloader: entry["bootloader"].bool, boardType: entry["boardType"].string)
    }
}

func firmwareJob(_ view: JSON?) -> FirmwareJob? {
    guard let view, view["class"].string == "FirmwareUpgrade" else { return nil }
    return FirmwareJob(
        phase: view["phase"].string.ifBlank("idle"),
        busy: view["busy"].bool,
        cancellable: view["cancellable"].bool,
        progress: view["progress"].double(0),
        messages: view["messages"].array.map(\.string),
        error: view["error"].string,
        choices: view["choices"].objects.map { FirmwareBuildChoice(name: $0["name"].string, url: $0["url"].string) },
        bestChoice: view["bestChoice"].int ?? -1,
        updateAvailable: view["updateAvailable"].string,
        px4StableVersion: view["px4StableVersion"].string,
        px4BetaVersion: view["px4BetaVersion"].string
    )
}

func cancelledOnLeave(_ phase: String?) -> Bool { phase == "connecting" || phase == "choosing" }

func firmwarePhaseText(_ phase: String) -> String {
    switch phase {
    case "connecting": "Waiting for the bootloader"
    case "choosing": "Choose the firmware build"
    case "erasing": "Erasing"
    case "programming": "Programming"
    case "verifying": "Verifying"
    case "complete": "Upgrade complete"
    case "failed": "Upgrade failed"
    default: ""
    }
}

let FIRMWARE_FROM_FILE = "file"
let FLASH_FAIL_TEXT = "If upgrade failed, make sure to connect directly to a powered USB port on your computer, not through a USB hub. Also make sure you are only powered via USB not battery."

private let APM_VEHICLES = ["copter", "heli", "plane", "rover", "sub"]

private let FIXED_FIRMWARE_SOURCES: [(first: String, second: String)] = [
    (FIRMWARE_FROM_FILE, "A firmware file"),
    ("px4:stable", "PX4 Pro, stable"),
    ("px4:beta", "PX4 Pro, beta"),
    ("px4:dev", "PX4 Pro, dev"),
    ("sik:stable", "SiK radio, stable"),
]

private let ARDUPILOT_FIRMWARE_SOURCES: [(first: String, second: String)] = APM_VEHICLES.flatMap { vehicle -> [(first: String, second: String)] in
    ["stable", "beta", "dev"].map { build -> (first: String, second: String) in
        (first: "ardupilot:\(vehicle):\(build)", second: "ArduPilot \(vehicle.capitalizedFirst), \(build)")
    }
}

let FIRMWARE_SOURCES: [(first: String, second: String)] = FIXED_FIRMWARE_SOURCES + ARDUPILOT_FIRMWARE_SOURCES

let DEFAULT_FIRMWARE_SOURCE = "px4:stable"
private let FIRMWARE_TYPE_PX4 = 12
private let FIRMWARE_TYPE_APM = 3
let DEFAULT_FIRMWARE_TYPE = "defaultFirmwareType"
let APM_VEHICLE_TYPE = "apmVehicleType"

struct FirmwareSourceSetting: Equatable {
    let name: String
    let value: Int
}

func rememberedSource(_ firmwareType: Int?, _ apmVehicleType: Int?) -> String {
    guard firmwareType == FIRMWARE_TYPE_APM else { return DEFAULT_FIRMWARE_SOURCE }
    let index = apmVehicleType ?? 0
    return "ardupilot:\(APM_VEHICLES.indices.contains(index) ? APM_VEHICLES[index] : APM_VEHICLES[0]):stable"
}

func sourceSettings(_ source: String) -> [FirmwareSourceSetting] {
    let parts = source.components(separatedBy: ":")
    switch parts.first {
    case "px4":
        return [FirmwareSourceSetting(name: DEFAULT_FIRMWARE_TYPE, value: FIRMWARE_TYPE_PX4)]
    case "ardupilot":
        let vehicle = parts.count > 1 ? APM_VEHICLES.firstIndex(of: parts[1]) : nil
        return [FirmwareSourceSetting(name: DEFAULT_FIRMWARE_TYPE, value: FIRMWARE_TYPE_APM)]
            + (vehicle.map { [FirmwareSourceSetting(name: APM_VEHICLE_TYPE, value: $0)] } ?? [])
    default:
        return []
    }
}

func firmwareSources(_ advanced: Bool) -> [(first: String, second: String)] {
    advanced ? FIRMWARE_SOURCES : FIRMWARE_SOURCES.filter { $0.first.hasSuffix(":stable") }
}

func sourceAfterAdvanced(_ source: String, _ advanced: Bool) -> String {
    if advanced || source.hasSuffix(":stable") { return source }
    guard let colon = source.lastIndex(of: ":"), colon > source.startIndex else { return DEFAULT_FIRMWARE_SOURCE }
    return "\(source[..<colon]):stable"
}

func flashingLabel(_ ports: [FirmwarePort], _ port: String) -> String {
    "Flashing - \(ports.first { $0.port == port }?.description.nonBlank ?? port)"
}

func firmwareChoice(_ source: String, _ file: String?) -> String? {
    source == FIRMWARE_FROM_FILE ? file : source
}

func firmwareFileAccepted(_ name: String) -> Bool {
    let lowered = name.lowercased()
    return FIRMWARE_EXTENSIONS.contains { lowered.hasSuffix(".\($0)") }
}

func firmwarePortSubtitle(_ entry: FirmwarePort) -> String {
    entry.bootloader ? "In its bootloader" : [entry.boardType, entry.port].filter { !$0.isBlank }.joined(separator: " \u{00b7} ")
}

// The core reads the file on its own thread after the picker has gone, so the chosen file is copied where the app
// can always read it, as Android stages it in its cache.
private func stagedFirmware(_ source: URL) -> URL? {
    let name = source.lastPathComponent
    guard firmwareFileAccepted(name) else { return nil }
    let target = FileManager.default.temporaryDirectory.appendingPathComponent(FIRMWARE_STAGE_PREFIX + name)
    let scoped = source.startAccessingSecurityScopedResource()
    defer { if scoped { source.stopAccessingSecurityScopedResource() } }
    try? FileManager.default.removeItem(at: target)
    return (try? FileManager.default.copyItem(at: source, to: target)) != nil ? target : nil
}

struct FirmwareScreen: View {
    @State private var job: FirmwareJob?
    @State private var ports: [FirmwarePort] = []
    @State private var port = ""
    @State private var file: URL?
    @State private var advanced = false
    @State private var source = DEFAULT_FIRMWARE_SOURCE
    @State private var sourceRestored = false
    @State private var refusal = ""
    @State private var flashingName = ""
    @State private var picking = false
    @State private var scope = ViewScope()
    @QgcBool(APM_FIRMWARE) private var apmVehicle
    @QgcFacts(FIRMWARE_UPGRADE_SETTINGS) private var upgradeSettings
    @Environment(\.theme) private var theme

    private var busy: Bool { job?.busy == true }
    private var stable: String { job?.px4StableVersion ?? "" }
    private var beta: String { job?.px4BetaVersion ?? "" }

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                note
                if !busy && ports.filter({ !$0.boardType.isBlank }).count > 1 {
                    Text(MULTIPLE_DEVICES)
                        .font(.bodyMedium)
                        .foregroundStyle(theme.aircast.warning)
                        .padding(.horizontal, Space.s4)
                }
                ForEach(ports) { entry in portRow(entry) }
                actions
                    .padding(.horizontal, Space.s4)
                    .padding(.vertical, Space.s3)
            }
        }
        .task {
            while !Task.isCancelled {
                let (readJob, readPorts) = await offMain { (firmwareJob(Qgc.get(FIRMWARE_VIEW)), firmwarePorts(Qgc.get(FIRMWARE_PORTS_VIEW))) }
                job = readJob
                if readPorts != ports && readJob?.busy != true {
                    let current = port
                    port = preselectedPort(readPorts, current) ?? (readPorts.contains(where: { $0.port == current }) ? current : "")
                }
                ports = readPorts
                try? await Task.sleep(for: .milliseconds(FIRMWARE_POLL_MS))
            }
        }
        .onChange(of: upgradeSettings, initial: true) { _, facts in restoreSource(facts) }
        .onDisappear {
            scope.cancel()
            if cancelledOnLeave(job?.phase) { offMainInOrder { _ = Qgc.invoke(FIRMWARE_CANCEL) } }
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.item]) { result in
            guard case .success(let chosen) = result else { return }
            scope.launch {
                let staged = await offMain { stagedFirmware(chosen) }
                refusal = staged == nil ? "Choose a .px4, .apj, .bin or .ihx firmware file." : ""
                file = staged ?? file
            }
        }
    }

    private var note: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            if let update = job?.updateAvailable, !update.isBlank {
                Text(update).font(.bodyMedium).foregroundStyle(theme.aircast.warning)
            }
            Text(ports.isEmpty ? "No USB serial devices attached" : "Flash a board over USB").font(.titleSmall)
            Text(
                "Plug in your device via USB, choose its port and either a release, downloaded once the board is "
                    + "identified, or a firmware file, then press Flash. A board "
                    + "running its firmware is asked to be unplugged and plugged back in so its bootloader starts."
            )
            .font(.bodyMedium)
        }
        .padding(Space.s4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme.colors.primaryContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }

    private func portRow(_ entry: FirmwarePort) -> some View {
        Button {
            port = entry.port
        } label: {
            HStack(spacing: Space.s4) {
                RadioIndicator(selected: entry.port == port)
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.description.ifBlank(entry.port)).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                    Text(firmwarePortSubtitle(entry)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, Space.s4)
            .padding(.vertical, Space.s2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(busy)
        .accessibilityAddTraits(entry.port == port ? .isSelected : [])
    }

    private func sourceTitle(_ entry: (first: String, second: String)) -> String {
        sourceLabel(entry.first, entry.second, stable, beta)
    }

    @ViewBuilder
    private var actions: some View {
        let offered = firmwareSources(advanced)
        let choice = firmwareChoice(source, file?.path)
        VStack(alignment: .leading, spacing: Space.s2) {
            ChoiceField(
                label: "Firmware",
                value: FIRMWARE_SOURCES.first(where: { $0.first == source }).map(sourceTitle) ?? source,
                options: offered.map(sourceTitle),
                enabled: !busy
            ) { index in
                if !busy && offered.indices.contains(index) { chooseSource(offered[index].first) }
            }
            if source.hasPrefix("ardupilot:"), let chibios = upgradeSettings.first(where: { $0.name == APM_CHIBIOS }) {
                FactRow(fact: chibios)
            }
            Toggle("Advanced settings", isOn: Binding(get: { advanced }, set: { checked in
                advanced = checked
                source = sourceAfterAdvanced(source, checked)
            }))
            .font(.bodyMedium)
            .disabled(busy)
            if bootloaderOffered(advanced, apmVehicle) {
                Button("Flash ChibiOS bootloader") { offMain { _ = Qgc.invoke(FLASH_BOOTLOADER) } }
                    .buttonStyle(.bordered)
                    .disabled(busy)
            }
            if source == FIRMWARE_FROM_FILE {
                Text(file?.lastPathComponent.removingPrefix(FIRMWARE_STAGE_PREFIX) ?? "No firmware file chosen").font(.bodyMedium)
            }
            if let warning = firmwareWarning(source) {
                Text(warning).font(.bodyMedium).foregroundStyle(theme.colors.error)
            }
            if busy {
                Text(flashingName.ifBlank(flashingLabel(ports, port))).font(.bodyMedium).lineLimit(1).truncationMode(.tail)
            }
            HStack(spacing: Space.s3) {
                Spacer(minLength: 0)
                if source == FIRMWARE_FROM_FILE {
                    Button("Choose file") { picking = true }
                        .buttonStyle(.bordered)
                        .disabled(busy)
                }
                Button("Flash") { flash(choice) }
                    .buttonStyle(.filled)
                    .disabled(busy || port.isBlank || choice == nil)
                if busy {
                    Button("Cancel") { run(FIRMWARE_CANCEL, []) }
                        .buttonStyle(.bordered)
                        .disabled(job?.cancellable != true)
                }
            }
            if let current = job {
                progress(current)
            }
            if !refusal.isBlank {
                Text(refusal).font(.bodyMedium).foregroundStyle(theme.colors.error)
            }
            if let messages = job?.messages, !messages.isEmpty {
                Text(messages.joined(separator: "\n"))
                    .font(.system(size: 12, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    @ViewBuilder
    private func progress(_ current: FirmwareJob) -> some View {
        if current.busy {
            ProgressView(value: min(max(current.progress, 0), 1))
        }
        let phase = firmwarePhaseText(current.phase)
        if !phase.isBlank {
            Text(phase).font(.bodyMedium)
        }
        if current.phase == "choosing" {
            ForEach(Array(current.choices.enumerated()), id: \.offset) { index, build in
                if index == current.bestChoice {
                    Button { run(FIRMWARE_CHOOSE, [build.url]) } label: { Text(build.name).frame(maxWidth: .infinity) }
                        .buttonStyle(.filled)
                } else {
                    Button { run(FIRMWARE_CHOOSE, [build.url]) } label: { Text(build.name).frame(maxWidth: .infinity) }
                        .buttonStyle(.bordered)
                }
            }
        }
        if !current.error.isBlank {
            Text(current.error).font(.bodyMedium).foregroundStyle(theme.colors.error).textSelection(.enabled)
        }
        if current.phase == "failed" {
            Text(FLASH_FAIL_TEXT).font(.bodyMedium)
        }
    }

    private func restoreSource(_ facts: [Fact]) {
        guard !sourceRestored, !facts.isEmpty else { return }
        let setting = { (name: String) -> Int? in facts.first { $0.name == name }?.value.int }
        source = rememberedSource(setting(DEFAULT_FIRMWARE_TYPE), setting(APM_VEHICLE_TYPE))
        sourceRestored = true
    }

    private func chooseSource(_ chosen: String) {
        source = chosen
        sourceRestored = true
        let writes = sourceSettings(chosen)
        offMain {
            for write in writes { _ = Qgc.writeRefusal("\(FIRMWARE_UPGRADE_SETTINGS).\(write.name)", write.value) }
        }
    }

    private func flash(_ choice: String?) {
        guard let chosen = choice, !port.isBlank else { return }
        flashingName = flashingLabel(ports, port)
        run(FIRMWARE_FLASH, [port, chosen])
    }

    private func run(_ path: String, _ args: [String]) {
        scope.launch {
            refusal = await offMain { Qgc.refusalOf(path, arguments: args) } ?? ""
        }
    }
}
#endif
