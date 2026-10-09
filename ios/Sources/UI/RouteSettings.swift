import SwiftUI

private let ALTITUDE_STEP = 1.0
private let SPEED_STEP = 0.5
private let FEET = "ft"
private let METRES_RANGE = 0.0...500.0
private let FEET_RANGE = 0.0...1640.0
private let ROUTE_ALTITUDE_NOTE = "New waypoints use this"
private let VEHICLE_SPEED_NOTE = "Vehicle default"

struct RouteAltitude: Equatable {
    let value: Double
    let units: String
    let path: String
}

func routeAltitude(_ plan: JSON?) -> RouteAltitude? {
    guard let control = plan?["defaults"]["altitude"], control.object != nil, let fact = factFromControl(control) else { return nil }
    return (fact.value.double ?? Double(fact.valueString)).map { RouteAltitude(value: $0, units: fact.units, path: fact.path) }
}

func routeSpeedRange(_ speed: SpeedSection) -> ClosedRange<Double> {
    speed.slider.flatMap { $0.from <= $0.to ? $0.from...$0.to : nil } ?? 0.0...30.0
}

struct RouteSettings: View {
    let plan: JSON?
    @Environment(\.theme) private var theme
    @State private var refusal: String?

    var body: some View {
        let altitude = routeAltitude(plan)
        let speed = speedSectionOf(plan?["defaults"]["flightSpeed"])
        VStack(alignment: .leading, spacing: 0) {
            Text("Route").font(.titleSmall).padding(.leading, 12).padding(.top, Space.s1)
            VStack(alignment: .leading, spacing: 0) {
                if let route = altitude {
                    SettingStepper(
                        label: "Altitude",
                        value: route.value,
                        unit: route.units,
                        step: ALTITUDE_STEP,
                        onSet: { wanted in write { Qgc.writeRefusal(route.path, wanted) } },
                        note: ROUTE_ALTITUDE_NOTE,
                        range: route.units == FEET ? FEET_RANGE : METRES_RANGE,
                        slider: true
                    )
                }
                if let section = speed {
                    SettingStepper(
                        label: "Speed",
                        value: section.value,
                        unit: section.units,
                        step: SPEED_STEP,
                        onSet: { wanted in write { Qgc.writeRefusal(section.specifyPath, true) ?? Qgc.writeRefusal(section.path, wanted) } },
                        note: section.specified ? nil : VEHICLE_SPEED_NOTE,
                        range: routeSpeedRange(section),
                        trailing: section.specified
                            ? { AnyView(PlanChip(label: "Auto", selected: false) { write { Qgc.writeRefusal(section.specifyPath, false) } }) }
                            : nil
                    )
                }
            }
            .padding(.horizontal, 12)
            MissionAltitudeFrame()
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.horizontal, 12)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func write(_ work: @escaping @Sendable () -> String?) {
        Task { refusal = await offMain(work) }
    }
}
