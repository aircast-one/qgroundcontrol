import Combine
import SwiftUI
import UIKit

struct PhoneBattery: Equatable {
    let percent: Int
    let charging: Bool
}

func phoneBattery(_ level: Float, _ state: UIDevice.BatteryState) -> PhoneBattery? {
    guard level >= 0 else { return nil }
    return PhoneBattery(percent: Int((level * 100).rounded()), charging: [.charging, .full].contains(state))
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
final class GcsBatteryMonitor: ObservableObject {
    @Published private(set) var reading: GcsBatteryReading?
    private var battery: PhoneBattery?
    private var changes: AnyCancellable?

    init() {
        UIDevice.current.isBatteryMonitoringEnabled = true
        changes = NotificationCenter.default.publisher(for: UIDevice.batteryLevelDidChangeNotification)
            .merge(with: NotificationCenter.default.publisher(for: UIDevice.batteryStateDidChangeNotification))
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in self?.refresh() }
        refresh()
    }

    private func refresh() {
        let device = UIDevice.current
        guard let current = phoneBattery(device.batteryLevel, device.batteryState), current != battery else { return }
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
                .minimumTouchTarget()
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
