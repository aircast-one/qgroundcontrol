import SwiftUI

let FIRMWARE_VIEW = "view.firmwareUpgrade"
private let RELEASES_POLL_MS = 2000

struct FirmwareReleases: Equatable {
    var updateAvailable = ""
    var px4StableVersion = ""
    var px4BetaVersion = ""
}

func firmwareReleases(_ view: JSON?) -> FirmwareReleases? {
    guard let view, view["class"].string == "FirmwareUpgrade" else { return nil }
    return FirmwareReleases(
        updateAvailable: view["updateAvailable"].string,
        px4StableVersion: view["px4StableVersion"].string,
        px4BetaVersion: view["px4BetaVersion"].string
    )
}

let PX4_RELEASES: [(first: String, second: String)] = [("px4:stable", "PX4 Pro, stable"), ("px4:beta", "PX4 Pro, beta")]

func px4Releases(_ advanced: Bool) -> [(first: String, second: String)] {
    advanced ? PX4_RELEASES : PX4_RELEASES.filter { $0.first.hasSuffix(":stable") }
}

let APM_FIRMWARE = "vehicle.apmFirmware"
let FLASH_BOOTLOADER = "vehicle.flashBootloader"

func bootloaderOffered(_ advanced: Bool, _ apmVehicle: Bool) -> Bool { advanced && apmVehicle }

let BETA_WARNING = "WARNING: BETA FIRMWARE. This firmware version is ONLY intended for beta testers. Although it has received FLIGHT TESTING, it represents actively changed code. Do NOT use for normal operation."
let DEV_WARNING = "WARNING: CONTINUOUS BUILD FIRMWARE. This firmware has NOT BEEN FLIGHT TESTED. It is only intended for DEVELOPERS. Run bench tests without props first. Do NOT fly this without additional safety precautions. Follow the forums actively when using it."

func firmwareWarning(_ source: String) -> String? {
    source.hasSuffix(":beta") ? BETA_WARNING : source.hasSuffix(":dev") ? DEV_WARNING : nil
}

func sourceLabel(_ source: String, _ label: String, _ stable: String, _ beta: String) -> String {
    if source == "px4:stable" && !stable.isBlank { return "PX4 Pro \(stable)" }
    if source == "px4:beta" && !beta.isBlank { return "PX4 Pro \(beta)" }
    return label
}

let USB_FLASHING_UNAVAILABLE = "Flashing needs a USB connection"
let USB_FLASHING_UNAVAILABLE_TEXT = "iOS does not let apps reach USB serial devices, so a board cannot be flashed from this device. "
    + "Flash it from Aircast on Android or from QGroundControl on a computer."

struct FirmwareScreen: View {
    @State private var releases: FirmwareReleases?
    @State private var advanced = false
    @QgcBool(APM_FIRMWARE) private var apmVehicle
    @Environment(\.theme) private var theme

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: Space.s2) {
                VStack(alignment: .leading, spacing: Space.s2) {
                    if let update = releases?.updateAvailable, !update.isBlank {
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
                releaseRows
                VStack(alignment: .leading, spacing: Space.s2) {
                    Toggle("Advanced settings", isOn: $advanced).font(.bodyMedium)
                    if bootloaderOffered(advanced, apmVehicle) {
                        Button("Flash ChibiOS bootloader") { offMain { _ = Qgc.invoke(FLASH_BOOTLOADER) } }
                            .buttonStyle(.bordered)
                    }
                }
                .padding(.horizontal, Space.s4)
                .padding(.vertical, Space.s3)
            }
        }
        .task {
            while !Task.isCancelled {
                releases = await offMain { firmwareReleases(Qgc.get(FIRMWARE_VIEW)) }
                try? await Task.sleep(for: .milliseconds(RELEASES_POLL_MS))
            }
        }
    }

    @ViewBuilder
    private var releaseRows: some View {
        let stable = releases?.px4StableVersion ?? ""
        let beta = releases?.px4BetaVersion ?? ""
        if !stable.isBlank || !beta.isBlank {
            SectionHeader(text: "Latest PX4 releases")
            ForEach(px4Releases(advanced), id: \.first) { source in
                SetupRow(title: sourceLabel(source.first, source.second, stable, beta), subtitle: firmwareWarning(source.first) ?? "")
            }
        }
    }
}
