import SwiftUI

struct PilotSetting: Equatable {
    let label: String
    let parameters: [String]
    var section: String
    let hint: String
    var whenUnlimited: String? = nil
}

private let RETURN_HOME = "Return to home"
private let FLIGHT_PROTECTION = "Flight protection"
private let FAILSAFES = "If something goes wrong"
private let FLIGHT_LIMITS = "Flight limits"

func pilotSettings(_ group: SettingsGroup) -> [PilotSetting] {
    switch group {
    case .Safety:
        return [
            PilotSetting(label: "Return-to-home altitude", parameters: ["RTL_RETURN_ALT", "RTL_ALT_M", "RTL_ALT"], section: RETURN_HOME, hint: "Climbs to this height before flying home."),
            PilotSetting(label: "Max altitude", parameters: ["GF_MAX_VER_DIST", "FENCE_ALT_MAX"], section: FLIGHT_PROTECTION, hint: "The aircraft will not climb above this.", whenUnlimited: "Most countries cap flights at 120 m."),
            PilotSetting(label: "Max distance", parameters: ["GF_MAX_HOR_DIST", "FENCE_RADIUS"], section: FLIGHT_PROTECTION, hint: "The aircraft will not fly farther from home than this."),
            PilotSetting(label: "Signal lost", parameters: ["NAV_DLL_ACT", "FS_GCS_ENABLE"], section: FAILSAFES, hint: "When the link to this app drops."),
            PilotSetting(label: "Remote controller lost", parameters: ["NAV_RCL_ACT", "FS_THR_ENABLE"], section: FAILSAFES, hint: "When the remote controller drops."),
            PilotSetting(label: "Low battery", parameters: ["COM_LOW_BAT_ACT", "BATT_FS_LOW_ACT"], section: FAILSAFES, hint: "When the battery runs low."),
        ]
    case .Control:
        return [
            PilotSetting(label: "Max horizontal speed", parameters: ["MPC_XY_VEL_MAX", "LOIT_SPEED_MS", "LOIT_SPEED"], section: FLIGHT_LIMITS, hint: "Fastest the aircraft flies forward and sideways."),
            PilotSetting(label: "Max climb speed", parameters: ["MPC_Z_VEL_MAX_UP", "PILOT_SPD_UP", "PILOT_SPEED_UP"], section: FLIGHT_LIMITS, hint: "Fastest the aircraft climbs."),
            PilotSetting(label: "Max descent speed", parameters: ["MPC_Z_VEL_MAX_DN", "PILOT_SPD_DN", "PILOT_SPEED_DN"], section: FLIGHT_LIMITS, hint: "Fastest the aircraft descends."),
        ]
    default:
        return []
    }
}

func pilotSearchHits(_ query: String) -> [PilotSetting] {
    let wanted = query.trimmed.lowercased()
    guard !wanted.isEmpty else { return [] }
    return SettingsGroup.allCases
        .flatMap { group in pilotSettings(group).map { setting in withChanges(setting) { $0.section = "\(group.title) \u{203a} \(setting.section)" } } }
        .filter { setting in ([setting.label, setting.section] + setting.parameters).contains { $0.lowercased().contains(wanted) } }
}

struct ShownPilotSetting: Equatable {
    let setting: PilotSetting
    let fact: Fact
}

private func labeled(_ fact: Fact, _ label: String) -> Fact {
    var copy = fact
    copy.shortLabel = label
    return copy
}

private func groupedInOrder<T>(_ items: [T], _ key: (T) -> String) -> [(key: String, value: [T])] {
    items.map(key).reduce([String]()) { seen, next in seen.contains(next) ? seen : seen + [next] }
        .map { section in (key: section, value: items.filter { key($0) == section }) }
}

func firstReported(_ setting: PilotSetting, _ reported: @escaping (String) -> Fact?) -> Fact? {
    let found = setting.parameters.lazy.map(reported).first { $0 != nil } ?? nil
    return found.map { labeled($0, setting.label) }
}

private struct ShownKey: Equatable {
    let settings: [PilotSetting]
    let setup: JSON?
}

struct PilotSettings: View {
    let settings: [PilotSetting]
    @HasVehicle private var hasVehicle
    @QgcPath(SETUP) private var setup
    @State private var shown: [ShownPilotSetting] = []

    init(group: SettingsGroup) {
        settings = pilotSettings(group)
    }

    init(settings: [PilotSetting]) {
        self.settings = settings
    }

    var body: some View {
        if settings.isEmpty {
            EmptyView()
        } else if !hasVehicle {
            OfflinePilotSettings(settings: settings, note: OFFLINE_PILOT_NOTE, offersLink: true)
        } else if !parametersReady(setup) {
            OfflinePilotSettings(settings: settings, note: LOADING_PILOT_NOTE, offersLink: false)
        } else {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(groupedInOrder(shown) { $0.setting.section }, id: \.key) { section in
                    SectionHeader(text: section.key)
                    ForEach(section.value, id: \.fact.path) { row in
                        PilotFactRow(shown: row.fact, setting: row.setting)
                    }
                }
            }
            .task(id: ShownKey(settings: settings, setup: setup)) {
                let settings = settings
                shown = await offMain {
                    settings.compactMap { setting in
                        firstReported(setting) { parameterFact($0) }.map { ShownPilotSetting(setting: setting, fact: $0) }
                    }
                }
            }
        }
    }
}

let OFFLINE_PILOT_NOTE = "Connect the aircraft to see and change these."
let LOADING_PILOT_NOTE = "Loading these from the aircraft\u{2026}"
private let OFFLINE_VALUE = "\u{2014}"
private let NOTE_ROW_MIN_HEIGHT: CGFloat = 48

func pilotSections(_ settings: [PilotSetting]) -> [(key: String, value: [PilotSetting])] {
    groupedInOrder(settings) { $0.section }
}

private struct OfflinePilotSettings: View {
    let settings: [PilotSetting]
    let note: String
    let offersLink: Bool
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(note)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .frame(maxWidth: .infinity, alignment: .leading)
                if offersLink {
                    Button("Add a link") { navigation.settingsPage = CONNECTIONS_PAGE }
                        .buttonStyle(.text)
                }
            }
            .padding(EdgeInsets(top: Space.s3, leading: Space.s4, bottom: 0, trailing: Space.s2))
            .frame(minHeight: NOTE_ROW_MIN_HEIGHT)
            ForEach(pilotSections(settings), id: \.key) { section in
                SectionHeader(text: section.key)
                ForEach(section.value, id: \.label) { setting in
                    OfflineRow(label: setting.label, hint: setting.hint)
                }
            }
        }
    }
}

private struct OfflineRow: View {
    let label: String
    let hint: String?
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s3) {
            VStack(alignment: .leading, spacing: 2) {
                Text(label)
                    .font(.bodyLarge)
                    .fontWeight(.medium)
                    .foregroundStyle(theme.colors.onSurface.opacity(DISABLED_ALPHA))
                if let hint {
                    Text(hint).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant.opacity(DISABLED_ALPHA))
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(OFFLINE_VALUE).font(.bodyLarge).foregroundStyle(theme.colors.onSurfaceVariant.opacity(DISABLED_ALPHA))
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
        .frame(minHeight: 56)
    }
}

private let OFFLINE_SENSORS = ["Compass", "Accelerometer", "Gyroscope"]

private let SENSOR_CHECKS = ["compass", "accelerometer", "gyro"]
private let SENSORS_SECTION = "Sensors"
private let CALIBRATED = "Calibrated"

func sensorChecks(_ state: CalibrationState?) -> [CalibrationRoutine] {
    SENSOR_CHECKS.compactMap { id in state?.routines.first { $0.id == id } }
}

func sensorHealthy(_ routine: CalibrationRoutine) -> Bool { routine.status == CALIBRATED }

struct SensorChecks: View {
    let onCalibrate: () -> Void
    @QgcPath(CALIBRATION) private var json
    @HasVehicle private var hasVehicle
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !hasVehicle {
                SectionHeader(text: SENSORS_SECTION)
                ForEach(OFFLINE_SENSORS, id: \.self) { OfflineRow(label: $0, hint: nil) }
            } else {
                let checks = sensorChecks(calibrationState(json))
                if !checks.isEmpty {
                    SectionHeader(text: SENSORS_SECTION)
                    ForEach(checks, id: \.id) { routine in
                        HStack(spacing: Space.s2) {
                            Text(sentenceCase(routine.title))
                                .font(.bodyLarge)
                                .fontWeight(.medium)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            Text(sensorHealthy(routine) ? "Normal" : routine.status.ifBlank("Not checked"))
                                .font(.bodyMedium)
                                .foregroundStyle(sensorHealthy(routine) ? theme.colors.onSurfaceVariant : theme.colors.error)
                            if !sensorHealthy(routine) {
                                Button("Calibrate", action: onCalibrate)
                                    .buttonStyle(.text)
                                    .disabled(!routine.enabled)
                            }
                        }
                        .padding(.leading, Space.s4)
                        .padding(.trailing, Space.s1)
                        .frame(minHeight: 56)
                    }
                    SetupRow(title: "Calibrate sensors", status: "", onClick: onCalibrate)
                }
            }
        }
    }
}

private struct RowKey: Equatable {
    let name: String
    let live: JSON?
    let revision: Int
}

private struct PilotFactRow: View {
    let shown: Fact
    let setting: PilotSetting
    @State private var revision = 0
    @QgcPath private var live: JSON?
    @State private var fact: Fact

    init(shown: Fact, setting: PilotSetting) {
        self.shown = shown
        self.setting = setting
        _live = QgcPath(parameterPath(shown.name))
        _fact = State(initialValue: shown)
    }

    var body: some View {
        FactRow(
            fact: pilotChoices(fact),
            subtitle: setting.hint,
            warning: factNumber(fact) == 0 ? setting.whenUnlimited : nil,
            onWrite: { revision += 1 }
        )
        .task(id: RowKey(name: shown.name, live: live, revision: revision)) {
            let name = shown.name
            let label = shown.shortLabel
            let read = await offMain { parameterFact(name).map { labeled($0, label) } }
            fact = read ?? shown
        }
    }
}

private let PILOT_CHOICES = [
    "Disabled": "Do nothing",
    "Hold mode": "Hover",
    "Loiter mode": "Hover",
    "Return mode": "Return home",
    "Land mode": "Land",
    "Warning": "Warn only",
    "Return at critical level, land at emergency level": "Return home, land if critical",
    "Terminate": "Stop motors",
]

func pilotChoice(_ label: String) -> String { PILOT_CHOICES[label] ?? label }

func pilotChoices(_ fact: Fact) -> Fact {
    var copy = fact
    copy.enumStrings = fact.enumStrings.map(pilotChoice)
    return copy
}

private let PILOT_LABELS = [
    "guidedMinimumAltitude": "Lowest altitude for takeoff and fly-to",
    "guidedMaximumAltitude": "Highest altitude for takeoff and fly-to",
    "maxGoToLocationDistance": "Max fly-to distance",
    "forwardFlightGoToLocationLoiterRad": "Circle radius after fly-to (planes)",
    "goToLocationRequiresConfirmInGuided": "Confirm fly-to first",
    "updateHomePosition": "Home follows this phone",
    "adsbServerConnectEnabled": "Show nearby aircraft (ADS-B)",
    "adsbServerHostAddress": "ADS-B server address",
    "adsbServerPort": "ADS-B server port",
    "displayPresetsTabFirst": "Open Plan on templates",
    "useConditionGate": "Use gate commands in survey patterns",
    "takeoffItemNotRequired": "Missions can start without a takeoff",
    "allowMultipleLandingPatterns": "Allow several landing paths",
    "vtolTransitionDistance": "VTOL transition distance",
    "keepMapCenteredOnVehicle": "Keep the aircraft centred on the map",
    "showAdditionalIndicatorsCompass": "Extra compass markers",
    "lockNoseUpCompass": "North-up compass",
    "showObstacleDistanceOverlay": "Show obstacle distances",
    "showPhotoVideoControl": "Show the shutter button",
    "showSimpleCameraControl": "Simple camera trigger",
    "showLogReplayStatusBar": "Show the log replay bar",
    "requestControlAllowTakeover": "Let other stations take control",
    "requestControlTimeout": "Control request timeout",
    "valueDisplay": "Battery shows",
    "threshold1": "Battery warning level",
    "threshold2": "Battery critical level",
    "consolidateMultipleBatteries": "Combine multiple batteries",
    "streamEnabled": "Show video",
    "videoFit": "Video fit",
    "gridLines": "Grid lines",
    "recordingFormat": "Recording format",
    "maxVideoSize": "Storage limit",
    "rtspTimeout": "Connection timeout",
    "forceVideoDecoder": "Video decoder",
    "aspectRatio": "Aspect ratio",
    "defaultMissionItemAltitude": "Default waypoint altitude",
    "offlineEditingFirmwareClass": "Plan offline for firmware",
    "offlineEditingVehicleClass": "Plan offline for aircraft type",
    "offlineEditingCruiseSpeed": "Planning cruise speed",
    "offlineEditingHoverSpeed": "Planning hover speed",
    "offlineEditingAscentSpeed": "Planning climb speed",
    "offlineEditingDescentSpeed": "Planning descent speed",
    "followTarget": "Send this phone's location (Follow Me)",
    "enableMultiVehiclePanel": "Show the multi-aircraft panel",
    "androidDontSaveToSDCard": "Save data to the SD card",
    "disableAllPersistence": "Don't save any data",
]

func pilotWorded(_ fact: Fact) -> Fact {
    PILOT_LABELS[fact.name].map { labeled(fact, $0) } ?? fact
}
