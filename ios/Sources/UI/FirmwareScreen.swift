import SwiftUI

let FIRMWARE_VIEW = "view.firmwareUpgrade"
let FIRMWARE_PORTS_VIEW = "view.firmwarePorts"
let FIRMWARE_FLASH = "firmware.flash"
let FIRMWARE_CANCEL = "firmware.cancel"
let FIRMWARE_CHOOSE = "firmware.choose"
let FIRMWARE_UPGRADE_SETTINGS = "settings.firmwareUpgradeSettings"
let APM_CHIBIOS = "apmChibiOS"
let FIRMWARE_POLL_MS = 500
let FIRMWARE_EXTENSIONS = ["px4", "apj", "bin", "ihx"]

struct FirmwarePort: Equatable {
    var port: String
    var description: String
    var bootloader: Bool
    var boardType: String = ""
}

let MULTIPLE_DEVICES = "Multiple devices detected. Make sure to select the correct one from the list."

func preselectedPort(_ ports: [FirmwarePort], _ current: String) -> String? {
    ports.first { $0.port == current && !$0.boardType.isBlank }?.port
        ?? (ports.first { $0.boardType == "Pixhawk" } ?? ports.first { $0.boardType == "SiK Radio" })?.port
}

struct FirmwareJob: Equatable {
    var phase: String
    var busy: Bool
    var cancellable: Bool
    var progress: Float
    var messages: [String]
    var error: String
    var choices: [(first: String, second: String)] = []
    var bestChoice: Int = -1
    var updateAvailable: String = ""
    var px4StableVersion: String = ""
    var px4BetaVersion: String = ""

    static func == (a: FirmwareJob, b: FirmwareJob) -> Bool {
        a.phase == b.phase && a.busy == b.busy && a.cancellable == b.cancellable && a.progress == b.progress
            && a.messages == b.messages && a.error == b.error
            && a.choices.map { [$0.first, $0.second] } == b.choices.map { [$0.first, $0.second] }
            && a.bestChoice == b.bestChoice && a.updateAvailable == b.updateAvailable
            && a.px4StableVersion == b.px4StableVersion && a.px4BetaVersion == b.px4BetaVersion
    }
}

func firmwarePorts(_ view: JSON?) -> [FirmwarePort] {
    (view?["ports"].arrayOrNil ?? []).filter { $0.object != nil && !$0["port"].string.isBlank }.map {
        FirmwarePort(port: $0["port"].string, description: $0["description"].string, bootloader: $0["bootloader"].bool, boardType: $0["boardType"].string)
    }
}

func firmwareJob(_ view: JSON?) -> FirmwareJob? {
    guard let view, view["class"].string == "FirmwareUpgrade" else { return nil }
    return FirmwareJob(
        phase: view["phase"].string.ifBlank("idle"),
        busy: view["busy"].bool,
        cancellable: view["cancellable"].bool,
        progress: Float(view["progress"].double(0)),
        messages: view["messages"].array.map(\.string),
        error: view["error"].string,
        choices: view["choices"].array.filter { $0.object != nil }.map { choice -> (first: String, second: String) in (choice["name"].string, choice["url"].string) },
        bestChoice: view["bestChoice"].isNull ? -1 : view["bestChoice"].int(0),
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

let FIRMWARE_SOURCES: [(first: String, second: String)] =
    [(FIRMWARE_FROM_FILE, "A firmware file"), ("px4:stable", "PX4 Pro, stable"), ("px4:beta", "PX4 Pro, beta"), ("px4:dev", "PX4 Pro, dev"), ("sik:stable", "SiK radio, stable")]
    + APM_VEHICLES.flatMap { vehicle in
        ["stable", "beta", "dev"].map { build -> (first: String, second: String) in ("ardupilot:\(vehicle):\(build)", "ArduPilot \(vehicle.capitalizedFirst), \(build)") }
    }

let DEFAULT_FIRMWARE_SOURCE = "px4:stable"
private let FIRMWARE_TYPE_PX4 = 12
private let FIRMWARE_TYPE_APM = 3
private let DEFAULT_FIRMWARE_TYPE = "defaultFirmwareType"
private let APM_VEHICLE_TYPE = "apmVehicleType"

func rememberedSource(_ firmwareType: Int?, _ apmVehicleType: Int?) -> String {
    guard firmwareType == FIRMWARE_TYPE_APM else { return DEFAULT_FIRMWARE_SOURCE }
    let index = apmVehicleType ?? 0
    return "ardupilot:\(APM_VEHICLES.indices.contains(index) ? APM_VEHICLES[index] : APM_VEHICLES[0]):stable"
}

func sourceSettings(_ source: String) -> [(first: String, second: Int)] {
    let parts = source.split(separator: ":", omittingEmptySubsequences: false).map(String.init)
    switch parts.first {
    case "px4": return [(DEFAULT_FIRMWARE_TYPE, FIRMWARE_TYPE_PX4)]
    case "ardupilot":
        let vehicle = parts.count > 1 ? APM_VEHICLES.firstIndex(of: parts[1]) : nil
        let stack: [(first: String, second: Int)] = [(DEFAULT_FIRMWARE_TYPE, FIRMWARE_TYPE_APM)]
        let vehicleType: [(first: String, second: Int)] = vehicle.map { [(APM_VEHICLE_TYPE, $0)] } ?? []
        return stack + vehicleType
    default: return []
    }
}

let APM_FIRMWARE = "vehicle.apmFirmware"
let FLASH_BOOTLOADER = "vehicle.flashBootloader"

func bootloaderOffered(_ advanced: Bool, _ apmVehicle: Bool) -> Bool { advanced && apmVehicle }

func firmwareSources(_ advanced: Bool) -> [(first: String, second: String)] {
    advanced ? FIRMWARE_SOURCES : FIRMWARE_SOURCES.filter { $0.first.hasSuffix(":stable") }
}

func sourceAfterAdvanced(_ source: String, _ advanced: Bool) -> String {
    if advanced || source.hasSuffix(":stable") { return source }
    let base = source.lastIndex(of: ":").map { String(source[..<$0]) } ?? ""
    return base.isEmpty ? DEFAULT_FIRMWARE_SOURCE : "\(base):stable"
}

let BETA_WARNING = "WARNING: BETA FIRMWARE. This firmware version is ONLY intended for beta testers. Although it has received FLIGHT TESTING, it represents actively changed code. Do NOT use for normal operation."
let DEV_WARNING = "WARNING: CONTINUOUS BUILD FIRMWARE. This firmware has NOT BEEN FLIGHT TESTED. It is only intended for DEVELOPERS. Run bench tests without props first. Do NOT fly this without additional safety precautions. Follow the forums actively when using it."

func firmwareWarning(_ source: String) -> String? {
    source.hasSuffix(":beta") ? BETA_WARNING : source.hasSuffix(":dev") ? DEV_WARNING : nil
}

func flashingLabel(_ ports: [FirmwarePort], _ port: String) -> String {
    "Flashing - \((ports.first { $0.port == port }?.description).flatMap { $0.isBlank ? nil : $0 } ?? port)"
}

func sourceLabel(_ source: String, _ label: String, _ stable: String, _ beta: String) -> String {
    if source == "px4:stable" && !stable.isBlank { return "PX4 Pro \(stable)" }
    if source == "px4:beta" && !beta.isBlank { return "PX4 Pro \(beta)" }
    return label
}

func firmwareChoice(_ source: String, _ file: String?) -> String? {
    source == FIRMWARE_FROM_FILE ? file : source
}

func firmwareFileAccepted(_ name: String) -> Bool {
    FIRMWARE_EXTENSIONS.contains { name.lowercased().hasSuffix(".\($0)") }
}

let USB_FLASHING_UNAVAILABLE = "Flashing needs a USB connection"
let USB_FLASHING_UNAVAILABLE_TEXT = "iOS does not let apps reach USB serial devices, so a board cannot be flashed from this device. "
    + "Flash it from Aircast on Android or from QGroundControl on a computer."

struct FirmwareScreen: View {
    @State private var job: FirmwareJob?
    @State private var advanced = false
    @QgcBool(APM_FIRMWARE) private var apmVehicle
    @Environment(\.theme) private var theme

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: Space.s2) {
                VStack(alignment: .leading, spacing: Space.s2) {
                    if let update = job?.updateAvailable, !update.isBlank {
                        Text(update).font(.bodyMedium).foregroundStyle(theme.aircast.warning)
                    }
                    Text(USB_FLASHING_UNAVAILABLE).font(.titleSmall)
                    Text(USB_FLASHING_UNAVAILABLE_TEXT).font(.bodyMedium)
                }
                .padding(Space.s4)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme.colors.primaryContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
                .padding(.horizontal, Space.s4)
                .padding(.vertical, Space.s2)
                releases
                VStack(alignment: .leading, spacing: Space.s2) {
                    Toggle("Advanced settings", isOn: $advanced).font(.bodyMedium).disabled(job?.busy == true)
                    if bootloaderOffered(advanced, apmVehicle) {
                        Button("Flash ChibiOS bootloader") { offMain { _ = Qgc.invoke(FLASH_BOOTLOADER) } }
                            .buttonStyle(.bordered)
                            .disabled(job?.busy == true)
                    }
                }
                .padding(.horizontal, Space.s4)
                .padding(.vertical, Space.s3)
            }
        }
        .task {
            while !Task.isCancelled {
                job = await offMain { firmwareJob(Qgc.get(FIRMWARE_VIEW)) }
                try? await Task.sleep(for: .milliseconds(FIRMWARE_POLL_MS))
            }
        }
    }

    @ViewBuilder
    private var releases: some View {
        let stable = job?.px4StableVersion ?? ""
        let beta = job?.px4BetaVersion ?? ""
        if !stable.isBlank || !beta.isBlank {
            SectionHeader(text: "Latest PX4 releases")
            ForEach(firmwareSources(advanced).filter { $0.first == "px4:stable" || $0.first == "px4:beta" }, id: \.first) { source in
                SetupRow(title: sourceLabel(source.first, source.second, stable, beta), subtitle: firmwareWarning(source.first) ?? "")
            }
        }
    }
}
