import SwiftUI

struct PilotSetting: Equatable {
    let label: String
    let parameters: [String]
    var section: String
    let hint: String
    var whenUnlimited: String? = nil
    var quantity: String? = nil
    var reportedOnly = false
}

private let RETURN_HOME = "Return home"
private let FLIGHT_PROTECTION = "Flight protection"
private let FAILSAFES = "If something goes wrong"
private let FLIGHT_LIMITS = "Flight limits"
private let ALTITUDE = "altitude"
private let DISTANCE = "distance"
private let SPEED = "speed"

private let ARDUPILOT_MAX_BATTERIES = 16
private let NUMBERED_BATTERY_PREFIXES = 9

func batteryPrefix(_ index: Int) -> String {
    index == 0 ? "BATT"
        : index < NUMBERED_BATTERY_PREFIXES ? "BATT\(index + 1)"
        : "BATT\(Character(UnicodeScalar(UInt8(ascii: "A") + UInt8(index - NUMBERED_BATTERY_PREFIXES))))"
}

private func batteryActions(_ index: Int) -> [PilotSetting] {
    [
        PilotSetting(label: "Battery \(index + 1) low", parameters: ["\(batteryPrefix(index))_FS_LOW_ACT"], section: FAILSAFES, hint: "When battery \(index + 1) runs low.", reportedOnly: true),
        PilotSetting(label: "Battery \(index + 1) critical", parameters: ["\(batteryPrefix(index))_FS_CRT_ACT"], section: FAILSAFES, hint: "When battery \(index + 1) is nearly empty.", reportedOnly: true),
    ]
}

func pilotSettings(_ group: SettingsGroup) -> [PilotSetting] {
    switch group {
    case .Safety:
        return [
            PilotSetting(label: "Return altitude", parameters: ["RTL_RETURN_ALT", "RTL_ALT_M", "RTL_ALT"], section: RETURN_HOME, hint: "Climbs to this height before flying home.", quantity: ALTITUDE),
            PilotSetting(label: "Max altitude", parameters: ["GF_MAX_VER_DIST", "FENCE_ALT_MAX"], section: FLIGHT_PROTECTION, hint: "The aircraft will not climb above this.", whenUnlimited: "Most countries cap flights at 120 m.", quantity: ALTITUDE),
            PilotSetting(label: "Max distance", parameters: ["GF_MAX_HOR_DIST", "FENCE_RADIUS"], section: FLIGHT_PROTECTION, hint: "The aircraft will not fly farther from home than this.", quantity: DISTANCE),
            PilotSetting(label: "Signal lost", parameters: ["NAV_DLL_ACT", "FS_GCS_ENABLE", "FS_GCS_ENABL"], section: FAILSAFES, hint: "When the link to this app drops."),
            PilotSetting(label: "Remote controller lost", parameters: ["NAV_RCL_ACT", "FS_THR_ENABLE"], section: FAILSAFES, hint: "When the remote controller drops."),
            PilotSetting(label: "Battery low", parameters: ["COM_LOW_BAT_ACT", "BATT_FS_LOW_ACT"], section: FAILSAFES, hint: "When the battery runs low."),
            PilotSetting(label: "Battery critical", parameters: ["BATT_FS_CRT_ACT"], section: FAILSAFES, hint: "When the battery is nearly empty.", reportedOnly: true),
        ] + (1..<ARDUPILOT_MAX_BATTERIES).flatMap(batteryActions)
    case .Control:
        return [
            PilotSetting(label: "Max horizontal speed", parameters: ["MPC_XY_VEL_MAX", "LOIT_SPEED_MS", "LOIT_SPEED"], section: FLIGHT_LIMITS, hint: "Fastest the aircraft flies forward and sideways.", quantity: SPEED),
            PilotSetting(label: "Max climb speed", parameters: ["MPC_Z_VEL_MAX_UP", "PILOT_SPD_UP", "PILOT_SPEED_UP"], section: FLIGHT_LIMITS, hint: "Fastest the aircraft climbs.", quantity: SPEED),
            PilotSetting(label: "Max descent speed", parameters: ["MPC_Z_VEL_MAX_DN", "PILOT_SPD_DN", "PILOT_SPEED_DN"], section: FLIGHT_LIMITS, hint: "Fastest the aircraft descends.", whenUnlimited: "At 0 it descends as fast as it climbs.", quantity: SPEED),
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

struct PilotOwner: Equatable {
    let setting: PilotSetting
    let group: SettingsGroup
}

func pilotOwner(_ name: String) -> PilotOwner? {
    SettingsGroup.allCases.flatMap { group in pilotSettings(group).map { PilotOwner(setting: $0, group: group) } }.first { $0.setting.parameters.contains(name) }
}

func offlineSettings(_ settings: [PilotSetting]) -> [PilotSetting] {
    settings.filter { !$0.reportedOnly }
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

struct PilotShownKey: Equatable {
    let settings: [PilotSetting]
    let setup: JSON?
}

func loadPilotShown(_ settings: [PilotSetting]) async -> [ShownPilotSetting] {
    await offMain {
        settings.compactMap { setting in
            firstReported(setting) { parameterFact($0, setting.quantity) }.map { ShownPilotSetting(setting: setting, fact: $0) }
        }
    }
}

struct PilotSettingSections: View {
    let settings: [PilotSetting]
    let shown: [ShownPilotSetting]
    @HasVehicle private var hasVehicle
    @QgcPath(SETUP) private var setup
    @Environment(AppNavigationState.self) private var navigation

    var body: some View {
        if settings.isEmpty {
            EmptyView()
        } else if !hasVehicle || !parametersReady(setup) {
            let offline = !hasVehicle
            ForEach(Array(pilotSections(offlineSettings(settings)).enumerated()), id: \.element.key) { at, section in
                Section {
                    ForEach(section.value, id: \.label) { setting in
                        OfflineRow(label: setting.label, hint: setting.hint)
                    }
                } header: {
                    SectionHeader(text: section.key)
                } footer: {
                    if at == 0 {
                        HStack(alignment: .firstTextBaseline) {
                            Text(offline ? OFFLINE_PILOT_NOTE : LOADING_PILOT_NOTE)
                            if offline {
                                Button("Add a link") { navigation.settingsPage = CONNECTIONS_PAGE }
                                    .font(.footnote)
                            }
                        }
                    }
                }
            }
        } else {
            let wanted = Set(settings.map(\.label))
            ForEach(groupedInOrder(shown.filter { wanted.contains($0.setting.label) }) { $0.setting.section }, id: \.key) { section in
                Section {
                    ForEach(section.value, id: \.fact.path) { row in
                        PilotFactRow(shown: row.fact, setting: row.setting)
                    }
                } header: {
                    SectionHeader(text: section.key)
                }
            }
        }
    }
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
            OfflinePilotSettings(settings: offlineSettings(settings), note: OFFLINE_PILOT_NOTE, offersLink: true)
        } else if !parametersReady(setup) {
            OfflinePilotSettings(settings: offlineSettings(settings), note: LOADING_PILOT_NOTE, offersLink: false)
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
                        firstReported(setting) { parameterFact($0, setting.quantity) }.map { ShownPilotSetting(setting: setting, fact: $0) }
                    }
                }
            }
        }
    }
}

let OFFLINE_PILOT_NOTE = "Connect the aircraft to see and change these."
let LOADING_PILOT_NOTE = "Loading these from the aircraft\u{2026}"
private let OFFLINE_VALUE = "\u{2014}"
private let NOTE_ROW_MIN_HEIGHT: CGFloat = Metrics.rowCompact

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
    @Environment(\.LocalNativeList) private var nativeList

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
        .rowPadding(horizontal: Space.s4, vertical: Space.s2)
        .frame(minHeight: nativeList ? nil : Metrics.row)
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
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            nativeSection
        } else {
            handDrawn
        }
    }

    @ViewBuilder private var nativeSection: some View {
        let checks = hasVehicle ? sensorChecks(calibrationState(json)) : []
        if !hasVehicle || !checks.isEmpty {
            Section {
                if !hasVehicle {
                    ForEach(OFFLINE_SENSORS, id: \.self) { OfflineRow(label: $0, hint: nil) }
                } else {
                    ForEach(checks, id: \.id) { routine in
                        LabeledContent {
                            HStack(spacing: Space.s3) {
                                Text(sensorHealthy(routine) ? "Normal" : routine.status.ifBlank("Not checked"))
                                    .foregroundStyle(sensorHealthy(routine) ? Color.secondary : theme.colors.error)
                                if !sensorHealthy(routine) {
                                    Button("Calibrate", action: onCalibrate)
                                        .buttonStyle(.borderless)
                                        .disabled(!routine.enabled)
                                }
                            }
                        } label: {
                            Text(sentenceCase(routine.title))
                        }
                    }
                    SetupRow(title: "Calibrate sensors", status: "", onClick: onCalibrate)
                }
            } header: {
                SectionHeader(text: SENSORS_SECTION)
            }
        }
    }

    private var handDrawn: some View {
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
                        .frame(minHeight: Metrics.row)
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
    @QgcBool("vehicle.fixedWing") private var fixedWing
    @QgcBool("vehicle.vtol") private var vtol
    @QgcBool("vehicle.rover") private var rover
    @QgcBool("vehicle.sub") private var sub

    init(shown: Fact, setting: PilotSetting) {
        self.shown = shown
        self.setting = setting
        _live = QgcPath(parameterPath(shown.name, setting.quantity))
        _fact = State(initialValue: shown)
    }

    var body: some View {
        FactRow(
            fact: pilotChoices(fact, holdWord(VehicleShape(fixedWing: fixedWing, vtol: vtol, rover: rover, sub: sub))),
            subtitle: setting.hint,
            warning: factNumber(fact) == 0 ? setting.whenUnlimited : nil,
            onWrite: { revision += 1 }
        )
        .task(id: RowKey(name: shown.name, live: live, revision: revision)) {
            let name = shown.name
            let label = shown.shortLabel
            let quantity = setting.quantity
            let read = await offMain { parameterFact(name, quantity).map { labeled($0, label) } }
            fact = read ?? shown
        }
    }
}

private let DO_NOTHING = "Do nothing"

struct PilotLinkRow: View {
    let name: String
    let owner: PilotOwner
    @QgcPath private var live: JSON?
    @State private var fact: Fact?
    @Environment(AppNavigationState.self) private var navigation
    @QgcBool("vehicle.fixedWing") private var fixedWing
    @QgcBool("vehicle.vtol") private var vtol
    @QgcBool("vehicle.rover") private var rover
    @QgcBool("vehicle.sub") private var sub

    init(name: String, owner: PilotOwner) {
        self.name = name
        self.owner = owner
        _live = QgcPath(parameterPath(name, owner.setting.quantity))
    }

    var body: some View {
        let hold = holdWord(VehicleShape(fixedWing: fixedWing, vtol: vtol, rover: rover, sub: sub))
        let status = fact.map { read in
            read.isEnum ? pilotChoice(read.enumStrings.indices.contains(read.enumIndex) ? read.enumStrings[read.enumIndex] : read.valueString, hold) : valueText(read)
        } ?? ""
        SetupRow(
            title: owner.setting.label,
            status: status,
            state: status == DO_NOTHING ? .NeedsAttention : .Neutral,
            onClick: { navigation.settingsPage = tabPage(owner.group) },
            subtitle: "In Settings \u{203a} \(owner.group.title)"
        )
        .task(id: RowKey(name: name, live: live, revision: 0)) {
            let name = name
            let quantity = owner.setting.quantity
            fact = await offMain { parameterFact(name, quantity) }
        }
    }
}

private let PILOT_CHOICES = [
    "Disabled": DO_NOTHING,
    "Disabled/NoAction": DO_NOTHING,
    "Return mode": "Return home",
    "Land mode": "Land",
    "Warning": "Warn only",
    "Return at critical level, land at emergency level": "Return home, land if critical",
    "Terminate": "Stop motors",
    "RTL": "Return home",
    "Enabled always RTL": "Return home",
    "Enabled always Land": "Land",
    "SmartRTL or RTL": "Retrace the path home, or return home",
    "Enabled always SmartRTL or RTL": "Retrace the path home, or return home",
    "SmartRTL or Land": "Retrace the path home, or land",
    "Enabled always SmartRTL or Land": "Retrace the path home, or land",
    "Brake or Land": "Brake, or land",
    "Enabled always Brake or Land": "Brake, or land",
    "Auto DO_LAND_START or RTL": "Fly the mission's landing, or return home",
    "Enabled Auto DO_LAND_START or RTL": "Fly the mission's landing, or return home",
    "Auto DO_LAND_START/DO_RETURN_PATH_START or RTL": "Fly the mission's landing or return path, or return home",
    "Enabled Auto DO_LAND_START/DO_RETURN_PATH_START or RTL": "Fly the mission's landing or return path, or return home",
]

private let HOLD_CHOICES: Set<String> = ["Hold mode", "Loiter mode"]

struct VehicleShape {
    var fixedWing = false
    var vtol = false
    var rover = false
    var sub = false
}

func holdWord(_ shape: VehicleShape) -> String {
    shape.vtol ? "Hover, or circle in forward flight" : shape.fixedWing ? "Circle" : shape.rover ? "Stop" : shape.sub ? "Hold position" : "Hover"
}

func pilotChoice(_ label: String, _ hold: String) -> String { HOLD_CHOICES.contains(label) ? hold : PILOT_CHOICES[label] ?? label }

func pilotChoices(_ fact: Fact, _ hold: String) -> Fact {
    withChanges(fact) { $0.enumStrings = fact.enumStrings.map { pilotChoice($0, hold) } }
}

private let PILOT_LABELS = [
    "guidedMinimumAltitude": "Lowest altitude for takeoff and fly-to",
    "guidedMaximumAltitude": "Highest altitude for takeoff and fly-to",
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
    "offlineEditingVehicleClass": "Plan offline for vehicle type",
    "offlineEditingCruiseSpeed": "Planning cruise speed",
    "offlineEditingHoverSpeed": "Planning hover speed",
    "offlineEditingAscentSpeed": "Planning climb speed",
    "offlineEditingDescentSpeed": "Planning descent speed",
    "followTarget": "Send this device's location (Follow Me)",
    "enableMultiVehiclePanel": "Show the multi-aircraft panel",
    "androidDontSaveToSDCard": "Save data to the SD card",
    "disableAllPersistence": "Don't save any data",
]

func pilotWorded(_ fact: Fact) -> Fact {
    PILOT_LABELS[fact.name].map { labeled(fact, $0) } ?? fact
}
