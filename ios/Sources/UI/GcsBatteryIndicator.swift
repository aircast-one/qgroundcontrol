import Combine
import SwiftUI
import UIKit

private let BATTERY_STATUS_CHARGING = 2
private let BATTERY_STATUS_DISCHARGING = 3
private let BATTERY_STATUS_FULL = 5
private let BATTERY_STATUS_UNKNOWN = 1

struct PhoneBattery: Equatable {
    let percent: Int
    let charging: Bool
}

func phoneBattery(_ level: Int, _ scale: Int, _ status: Int) -> PhoneBattery? {
    guard level >= 0, scale > 0 else { return nil }
    return PhoneBattery(percent: level * 100 / scale, charging: status == BATTERY_STATUS_CHARGING || status == BATTERY_STATUS_FULL)
}

func gcsBatteryPath(_ battery: PhoneBattery) -> String { "view.gcsBattery(\(battery.percent),\(battery.charging))" }

struct GcsBatteryReading: Equatable {
    let state: String
    let levelText: String
    let stateText: String
    let heading: String
    let title: String
}

func gcsBatteryReading(_ view: JSON?) -> GcsBatteryReading? {
    guard let it = view, it.has("state") else { return nil }
    return GcsBatteryReading(state: it["state"].string, levelText: it["levelText"].string, stateText: it["stateText"].string, heading: it["heading"].string, title: it["title"].string)
}

private func batteryColour(_ state: String, _ theme: Theme) -> Color {
    switch state {
    case "charging": theme.aircast.success
    case "critical": theme.colors.error
    case "low": theme.aircast.warning
    default: theme.colors.onSurfaceVariant
    }
}

@MainActor
private func devicePhoneBattery() -> PhoneBattery? {
    let device = UIDevice.current
    device.isBatteryMonitoringEnabled = true
    let status = switch device.batteryState {
    case .charging: BATTERY_STATUS_CHARGING
    case .full: BATTERY_STATUS_FULL
    case .unplugged: BATTERY_STATUS_DISCHARGING
    default: BATTERY_STATUS_UNKNOWN
    }
    return phoneBattery(device.batteryLevel < 0 ? -1 : Int((device.batteryLevel * 100).rounded()), 100, status)
}

@MainActor
final class GcsBatteryMonitor: ObservableObject {
    @Published private(set) var reading: GcsBatteryReading?
    private var battery: PhoneBattery?
    private var changes: AnyCancellable?

    init() {
        changes = NotificationCenter.default.publisher(for: UIDevice.batteryLevelDidChangeNotification)
            .merge(with: NotificationCenter.default.publisher(for: UIDevice.batteryStateDidChangeNotification))
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in self?.refresh() }
        refresh()
    }

    private func refresh() {
        guard let current = devicePhoneBattery(), current != battery else { return }
        battery = current
        offMain { [weak self] in
            let read = gcsBatteryReading(Qgc.get(gcsBatteryPath(current)))
            onMain {
                guard let self, self.battery == current else { return }
                self.reading = read
            }
        }
    }
}

@propertyWrapper
struct GcsBattery: DynamicProperty {
    @StateObject private var monitor = GcsBatteryMonitor()

    var wrappedValue: GcsBatteryReading? { monitor.reading }
}

struct GcsBatteryCell: View {
    @Environment(\.theme) private var theme
    let reading: GcsBatteryReading?
    @State private var open = false

    var body: some View {
        if let shown = reading {
            Text(shown.levelText)
                .font(.labelMedium)
                .foregroundStyle(batteryColour(shown.state, theme))
                .lineLimit(1)
                .onTapGesture { open = true }
                .accessibilityLabel("\(shown.title) \(shown.levelText)")
                .accessibilityHint(shown.heading)
                .accessibilityAddTraits(.isButton)
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            VStack(alignment: .leading, spacing: 0) {
                                Text(shown.heading).font(.titleMedium).padding(.horizontal, Space.s6).padding(.vertical, Space.s2)
                                StatusListItem(headline: "Charge") { Text(shown.levelText) }
                                StatusListItem(headline: "State") { Text(shown.stateText) }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.bottom, Space.s6)
                        }
                    }
                }
        }
    }
}
