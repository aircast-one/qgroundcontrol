import SwiftUI

let GPS_RTK_VIEW = "view.gpsRtk"
private let RTK_POLL_MS = 1000

struct RtkStatus: Equatable {
    var active: Bool
    var valid: Bool
    var satellites: Int?
    var durationS: Double?
    var accuracyM: Double?
    var accuracyText: String? = nil
    var latitude: Double? = nil
    var longitude: Double? = nil
    var altitudeM: Double? = nil
}

let RTK_SETTINGS_PAGE = "RTK GPS"
private let RTK_SETTINGS = "settings.rtkSettings"
private let RTK_AUTOCONNECT_GROUP = "settings.autoConnectSettings"
private let RTK_AUTOCONNECT = "autoConnectRTKGPS"

func basePositionWrites(_ status: RtkStatus) -> [(String, Double)]? {
    guard status.valid else { return nil }
    let writes = [
        ("fixedBasePositionLatitude", status.latitude),
        ("fixedBasePositionLongitude", status.longitude),
        ("fixedBasePositionAltitude", status.altitudeM),
        ("fixedBasePositionAccuracy", status.accuracyM),
    ]
    guard writes.allSatisfy({ $0.1 != nil }) else { return nil }
    return writes.map { ("\(RTK_SETTINGS).\($0.0)", $0.1!) }
}

func fixedBaseChosen(_ facts: [Fact]) -> Bool {
    let value = facts.first(where: { $0.name == "useFixedBasePosition" })?.value
    if case .number(let number)? = value, (1..<2).contains(number) { return true }
    return value == .bool(true) || value == .string("1")
}

private func number(_ json: JSON, _ key: String) -> Double? {
    json[key].isNull ? nil : json[key].double.flatMap { $0.isNaN ? nil : $0 }
}

func rtkStatus(_ view: JSON?) -> RtkStatus? {
    guard let it = view, it["connected"].bool else { return nil }
    return RtkStatus(
        active: it["active"].bool,
        valid: it["valid"].bool,
        satellites: number(it, "numSatellites").map { Int($0) },
        durationS: number(it, "currentDuration"),
        accuracyM: number(it, "currentAccuracy"),
        accuracyText: it["currentAccuracyText"].string.isEmpty ? nil : it["currentAccuracyText"].string,
        latitude: number(it, "currentLatitude"),
        longitude: number(it, "currentLongitude"),
        altitudeM: number(it, "currentAltitude")
    )
}

func rtkRows(_ status: RtkStatus) -> [(String, String)] {
    [
        ("Satellites", status.satellites.map(String.init) ?? ""),
        ("Duration", "\(String(format: "%.0f", status.durationS ?? 0)) s"),
        status.accuracyText.flatMap { (status.accuracyM ?? 0) > 0 ? (status.valid ? "Accuracy" : "Current Accuracy", $0) : nil },
    ].compactMap { $0 }
}

func rtkHeadline(_ status: RtkStatus) -> String { status.active ? "Survey-in Active" : "RTK Streaming" }

@MainActor
final class RtkStatusPoller: ObservableObject {
    @Published private(set) var status: RtkStatus?
    private var timer: Timer?

    init() {
        poll()
        let ticking = Timer(timeInterval: Double(RTK_POLL_MS) / 1000, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.poll() }
        }
        RunLoop.main.add(ticking, forMode: .common)
        timer = ticking
    }

    deinit { timer?.invalidate() }

    private func poll() {
        offMain { [weak self] in
            let read = rtkStatus(Qgc.get(GPS_RTK_VIEW))
            onMain {
                guard let self, self.status != read else { return }
                self.status = read
            }
        }
    }
}

@propertyWrapper
struct RtkStatusWatch: DynamicProperty {
    @StateObject private var poller = RtkStatusPoller()

    var wrappedValue: RtkStatus? { poller.status }
}

struct RtkSettingsSheetContent: View {
    let status: RtkStatus?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 6) {
                RtkSettingsSection(status: status)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, Space.s5)
            .padding(.bottom, Space.s6)
        }
    }
}

struct RtkIndicatorCell: View {
    let status: RtkStatus?
    @State private var open = false

    var body: some View {
        if let shown = status {
            Text("RTK")
                .font(.labelMedium)
                .onTapGesture { open = true }
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            ScrollView {
                                VStack(alignment: .leading, spacing: 6) {
                                    Text("RTK GPS Status").font(.titleSmall)
                                    Text(rtkHeadline(shown)).font(.bodyMedium)
                                    ForEach(rtkRows(shown), id: \.0) { label, value in
                                        Text("\(label)  \(value)").font(.bodySmall)
                                    }
                                    RtkSettingsSection(status: shown)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, Space.s5)
                                .padding(.bottom, Space.s6)
                            }
                        }
                    }
                }
        }
    }
}

struct RtkSettingsSection: View {
    let status: RtkStatus?
    @State private var reloads = 0
    @State private var facts: [Fact] = []
    @State private var autoConnect: Fact?

    var body: some View {
        let writes = status.flatMap(basePositionWrites)
        VStack(alignment: .leading, spacing: 6) {
            Text("RTK GPS Settings").font(.titleSmall)
            if let autoConnect {
                FactRow(fact: autoConnect, title: "AutoConnect", onWrite: { reloads += 1 })
            }
            ForEach(facts) { fact in
                FactRow(fact: fact, onWrite: { reloads += 1 })
            }
            if fixedBaseChosen(facts) {
                HStack {
                    Text("Current base position").frame(maxWidth: .infinity, alignment: .leading)
                    Button(writes != nil ? "Save" : "Not Yet Valid") {
                        let saving = writes ?? []
                        Task {
                            await offMain { saving.forEach { Qgc.set($0.0, $0.1) } }
                            reloads += 1
                        }
                    }
                    .buttonStyle(.bordered)
                    .disabled(writes == nil)
                }
            }
        }
        .task(id: reloads) {
            let read = await offMain {
                (
                    settingsSections(Qgc.get(settingsPagePath(RTK_SETTINGS_PAGE))).flatMap { section in section.blocks.flatMap(\.facts) },
                    Qgc.get("\(RTK_AUTOCONNECT_GROUP).\(RTK_AUTOCONNECT)")
                )
            }
            facts = read.0
            autoConnect = read.1.object.map { fields in
                Qgc.fact(RTK_AUTOCONNECT_GROUP, JSON.object(fields.merging(["name": .string(RTK_AUTOCONNECT)]) { _, named in named }))
            }
        }
    }
}
