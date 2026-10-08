import SwiftUI

let PLAN_TRANSFORM_VIEW = "view.planTransform"
let OFFSET_MISSION = "plan.missionController.offsetMission"
let REPOSITION_MISSION = "plan.missionController.repositionMission"
let ROTATE_MISSION = "plan.missionController.rotateMission"

func transformHome(_ view: JSON?) -> TrackPoint? {
    guard let home = view?["home"], home.object != nil else { return nil }
    return TrackPoint(latitude: home["latitude"].double(.nan), longitude: home["longitude"].double(.nan))
}

struct DistanceUnit: Equatable {
    let name: String
    let metresPerUnit: Double
}

private let METRES = DistanceUnit(name: "m", metresPerUnit: 1)

func transformUnit(_ view: JSON?, _ axis: String) -> DistanceUnit {
    guard let view else { return METRES }
    let unit = DistanceUnit(name: view["\(axis)Unit"].string.ifBlank("m"), metresPerUnit: view["\(axis)MetresPerUnit"].double(.nan))
    return unit.metresPerUnit.isFinite && unit.metresPerUnit > 0 ? unit : METRES
}

func offsetArgs(_ east: String, _ north: String, _ up: String, _ horizontal: DistanceUnit, _ vertical: DistanceUnit, takeoff: Bool, landing: Bool) -> [JSON]? {
    let values = [(east, horizontal), (north, horizontal), (up, vertical)].map { typed, unit in
        typedNumber(typed.ifBlank("0")).map { $0 * unit.metresPerUnit }
    }
    let known = values.compactMap { $0 }.filter(\.isFinite)
    return known.count == values.count ? known.map(JSON.number) + [.bool(takeoff), .bool(landing)] : nil
}

private func transformRefusal(_ path: String, _ args: [JSON]) -> String? {
    refusal(Qgc.call(path, arguments: args))
}

private struct NumberField: View {
    let label: String
    let value: String
    let onChange: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let invalid = !value.isBlank && typedNumber(value) == nil
        VStack(alignment: .leading, spacing: Space.s1) {
            Text(label).font(.bodySmall).foregroundStyle(invalid ? theme.colors.error : theme.colors.onSurfaceVariant)
            TextField(label, text: Binding(get: { value }, set: onChange))
                .keyboardType(.numbersAndPunctuation)
                .textFieldStyle(.roundedBorder)
        }
    }
}

private struct CheckRow: View {
    let label: String
    let checked: Bool
    let onChange: (Bool) -> Void

    var body: some View {
        Toggle(label, isOn: Binding(get: { checked }, set: onChange))
    }
}

struct PlanTransformDialog: View {
    let onDismiss: () -> Void
    @QgcPath(PLAN_TRANSFORM_VIEW) private var view
    @Environment(\.theme) private var theme
    @State private var east = "0"
    @State private var north = "0"
    @State private var up = "0"
    @State private var offsetTakeoff = false
    @State private var offsetLanding = false
    @State private var degrees = "0"
    @State private var rotateTakeoff = false
    @State private var rotateLanding = false
    @State private var repositioning = false
    @State private var refusal: String?

    var body: some View {
        let home = transformHome(view)
        let horizontal = transformUnit(view, "horizontal")
        let vertical = transformUnit(view, "vertical")
        PlanDialog(title: "Transform", onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Offset mission").font(.titleSmall)
                NumberField(label: "East (\(horizontal.name))", value: east) { east = $0 }
                NumberField(label: "North (\(horizontal.name))", value: north) { north = $0 }
                NumberField(label: "Up (\(vertical.name))", value: up) { up = $0 }
                CheckRow(label: "Also move takeoff items", checked: offsetTakeoff) { offsetTakeoff = $0 }
                CheckRow(label: "Also move landing items", checked: offsetLanding) { offsetLanding = $0 }
                Text("Note: Home altitude is not modified.").font(.bodySmall)
                let offset = offsetArgs(east, north, up, horizontal, vertical, takeoff: offsetTakeoff, landing: offsetLanding)
                Button("Apply offset") { if let offset { apply(OFFSET_MISSION, offset) } }
                    .buttonStyle(.bordered)
                    .disabled(offset == nil)

                Divider()
                Text("Reposition mission").font(.titleSmall)
                if home == nil { Text("Home position must be set to reposition the mission.").font(.bodySmall) }
                Button("Move to Position") { repositioning = true }
                    .buttonStyle(.bordered)
                    .disabled(home == nil)

                Divider()
                Text("Rotate mission").font(.titleSmall)
                if home == nil { Text("Home position must be set to rotate the mission.").font(.bodySmall) }
                NumberField(label: "Clockwise (deg)", value: degrees) { degrees = $0 }
                CheckRow(label: "Also move takeoff items", checked: rotateTakeoff) { rotateTakeoff = $0 }
                CheckRow(label: "Also move landing items", checked: rotateLanding) { rotateLanding = $0 }
                Text("Note: Complex items are rotated by moving their reference coordinate: their geometry and orientation are not changed.").font(.bodySmall)
                let rotation = typedNumber(degrees)
                Button("Apply rotation") { if let rotation { apply(ROTATE_MISSION, [.number(rotation), .bool(rotateTakeoff), .bool(rotateLanding)]) } }
                    .buttonStyle(.bordered)
                    .disabled(home == nil || rotation == nil)

                if let refusal { Text(refusal).foregroundStyle(theme.colors.error) }
            }
            .background {
                if repositioning, let home {
                    EditPositionDialog(
                        at: home,
                        onDismiss: { repositioning = false },
                        title: "Reposition mission",
                        confirm: "Move to Position",
                        vehicleConfirm: "Move to Vehicle Position"
                    ) { latitude, longitude in
                        repositioning = false
                        apply(REPOSITION_MISSION, [.object(["latitude": .number(latitude), "longitude": .number(longitude)])])
                    }
                }
            }
        } buttons: {
            Button("Close", action: onDismiss)
        }
    }

    private func apply(_ path: String, _ args: [JSON]) {
        Task { refusal = await offMain { transformRefusal(path, args) } }
    }
}
