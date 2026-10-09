import SwiftUI

private let CHIP_SIZE: CGFloat = 40
private let FEET = "ft"
private let ALTITUDE_CEILING_METRES = 500.0
private let ALTITUDE_CEILING_FEET = 1640.0
private let HOLD_CEILING_SECONDS = 3600.0
private let SPEED_STEP = 0.5
let SPEED_RANGE = 0.0...30.0

func altitudeRange(_ units: String) -> ClosedRange<Double> {
    0.0...(units == FEET ? ALTITUDE_CEILING_FEET : ALTITUDE_CEILING_METRES)
}

func stripRows(_ rows: [ItemRow]) -> [ItemRow] { rows.filter { $0.index != HOME_ITEM } }

struct WaypointStrip: View {
    let rows: [ItemRow]
    let selected: Int?
    let onPick: (Int) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let shown = stripRows(rows)
        ScrollViewReader { list in
            ScrollView(.horizontal, showsIndicators: false) {
                LazyHStack(spacing: Space.s2) {
                    ForEach(shown) { row in chip(row, chosen: row.index == selected) }
                }
                .padding(.horizontal, Space.s1)
                .padding(.vertical, 2)
            }
            .frame(height: CHIP_SIZE + 4)
            .onChange(of: selected, initial: true) { _, now in
                guard let now, shown.contains(where: { $0.index == now }) else { return }
                withAnimation { list.scrollTo(now) }
            }
        }
    }

    private func chip(_ row: ItemRow, chosen: Bool) -> some View {
        Button { onPick(row.index) } label: {
            Text(row.number)
                .font(.labelLarge)
                .lineLimit(1)
                .foregroundStyle(row.readyForSave ? theme.colors.surface : theme.aircast.warning)
                .frame(width: CHIP_SIZE, height: CHIP_SIZE)
                .background(row.readyForSave ? hexColour(row.colour) : Color.clear, in: Circle())
                .overlay {
                    if chosen {
                        Circle().strokeBorder(theme.colors.onSurface, lineWidth: 3)
                    } else if !row.readyForSave {
                        Circle().strokeBorder(theme.aircast.warning, lineWidth: 1)
                    }
                }
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .id(row.index)
        .accessibilityLabel("\(sentenceCase(row.name)) \(row.number)")
        .accessibilityAddTraits(chosen ? .isSelected : [])
    }
}

struct WaypointSettings: View {
    let item: MissionItem
    let globalFrame: Int?
    let onWrite: (String?, @escaping () -> Bool) -> Void
    @Environment(\.theme) private var theme
    @MapPath private var json: JSON?

    init(item: MissionItem, globalFrame: Int?, onWrite: @escaping (String?, @escaping () -> Bool) -> Void) {
        self.item = item
        self.globalFrame = globalFrame
        self.onWrite = onWrite
        _json = MapPath("view.itemFacts(\(item.index))")
    }

    var body: some View {
        let units = item.altitudeEditUnits.ifBlank("m")
        let index = item.index
        VStack(alignment: .leading, spacing: 0) {
            if !item.altitude.isNaN && index != HOME_ITEM {
                SettingStepper(
                    label: "Altitude",
                    value: item.altitude,
                    unit: units,
                    step: 1.0,
                    onSet: { metres in onWrite(nil) { PlanBridge.setAltitude(index, metres) } },
                    note: item.altitudeFrameText.isBlank ? nil : item.altitudeFrameText,
                    range: altitudeRange(units),
                    slider: true
                )
            }
            if let speed = waypointSpeed(json) {
                SettingStepper(
                    label: "Speed",
                    value: speed.value,
                    unit: speed.units,
                    step: SPEED_STEP,
                    onSet: { wanted in onWrite(nil) { setOk(speed.specifyPath, true) && setOk(speed.path, wanted) } },
                    note: speed.specified ? nil : "Mission speed",
                    range: SPEED_RANGE,
                    trailing: speed.specified
                        ? { AnyView(PlanChip(label: "Auto", selected: false) { onWrite("Using the mission speed") { setOk(speed.specifyPath, false) } }) }
                        : nil
                )
            }
            if let hold = waypointHold(json) {
                SettingStepper(
                    label: "Hold",
                    value: hold.seconds,
                    unit: hold.units,
                    step: 1.0,
                    onSet: { seconds in onWrite(nil) { setOk(hold.path, seconds) } },
                    range: 0.0...HOLD_CEILING_SECONDS
                )
            }
            if !item.altitude.isNaN && itemReferenceShown(globalFrame) && index != HOME_ITEM {
                HStack {
                    Text("Altitude mode").font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                    AltitudeModePicker(
                        item: item,
                        onPick: { raw in onWrite("Setting the altitude frame") { PlanBridge.setAltitudeMode(index, raw) } },
                        globalFrameMixed: itemReferenceSelectable(globalFrame)
                    )
                }
                .padding(.vertical, Space.s1)
            }
            if let leg = legText(item) {
                Text(leg).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.vertical, Space.s1)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
    }
}
