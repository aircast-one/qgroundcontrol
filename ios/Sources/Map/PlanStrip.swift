import SwiftUI

struct PlanStat: Equatable {
    let label: String
    let value: String
}

func planStats(_ itemCount: Int, _ items: [MissionItem], _ summary: JSON?) -> [PlanStat] {
    let stats: [PlanStat?] = [
        PlanStat(label: "Items", value: String(itemCount)),
        summaryRow(summary, "Distance").map { PlanStat(label: "Distance", value: $0) },
        summaryRow(summary, "Time").map { PlanStat(label: "Time", value: $0.removingPrefix(NO_HOURS)) },
        highestAltitude(items).map { PlanStat(label: "Max alt", value: $0) },
    ]
    return stats.compactMap { $0 }
}

func highestAltitude(_ items: [MissionItem]) -> String? {
    items.filter { $0.index != HOME_ITEM && !$0.altitude.isNaN && !$0.altitudeText.isBlank }
        .max { $0.altitude < $1.altitude }?
        .altitudeText
}

private let NO_HOURS = "00:"
let TAP_TO_ADD = "Tap the map to add a waypoint"
let TERRAIN_CONFLICT_HERE = "Too close to the terrain here"
private let PANEL_DRAG_THRESHOLD: CGFloat = 24

func advancedDetail(_ item: MissionItem) -> String? {
    let detail = [item.foldedCommands > 0 ? "Mission items \(sequenceLabel(item))" : nil, legText(item)]
        .compactMap { $0 }
        .joined(separator: " \u{00b7} ")
    return detail.isBlank ? nil : detail
}

func placedPattern(_ surveys: [Survey], _ wanted: Int) -> Survey? {
    surveys.first { $0.index == wanted } ?? (wanted == NEWEST_PATTERN ? surveys.max { $0.index < $1.index } : nil)
}

func appendSequence(_ items: [MissionItem]) -> Int {
    items.max { $0.index < $1.index }?.sequence ?? HOME_ITEM
}

func tapCloses(_ selected: MapHit?, _ panelOpen: Bool) -> Bool {
    guard let selected else { return false }
    if case .Waypoint = selected { return panelOpen }
    return true
}

func terrainWarning(_ legs: Int, _ items: Int) -> String? {
    legs > 0 ? "\(legs) \(legs == 1 ? "leg hits" : "legs hit") the terrain"
        : items > 0 ? "\(items) \(items == 1 ? "item hits" : "items hit") the terrain"
        : nil
}

extension View {
    func panelDrag(_ onOpen: @escaping (Bool) -> Void) -> some View {
        simultaneousGesture(
            DragGesture(minimumDistance: 10).onEnded { drag in
                let travelled = drag.translation.height
                guard abs(travelled) > abs(drag.translation.width) else { return }
                if travelled < -PANEL_DRAG_THRESHOLD { onOpen(true) } else if travelled > PANEL_DRAG_THRESHOLD { onOpen(false) }
            }
        )
    }
}

let FIRST_TAP_SETS_HOME = "The first tap also sets home."

func selectionTitle(_ selected: MapHit, _ items: [MissionItem]) -> String {
    switch selected {
    case .FenceVertex: "Fence corner"
    case .Circle, .CircleCentre, .CircleRadius: "Circular fence"
    case .ShapeCentre(let fence, let owner), .ShapeRadius(let fence, let owner): fence ? "Fence" : sentenceCase(patternName(owner, items))
    case .SurveyVertex(let item, _): "\(sentenceCase(patternName(item, items))) corner"
    case .Rally(let index): "Rally point \(index + 1)"
    case .BreachReturn: "Breach return point"
    case .LandingPlace: "Landing"
    default: "Selected"
    }
}

private struct StripIconButton: View {
    let icon: Icon
    let label: String
    var tint: Color? = nil
    let action: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: action) {
            Image(icon).frame(width: 48, height: 48).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .foregroundStyle(tint ?? theme.colors.onSurface)
        .accessibilityLabel(label)
    }
}

struct WaypointStripBar: View {
    let rows: [ItemRow]
    let altitudes: [Int: String]
    let conflicts: Set<Int>
    let selected: Int?
    let onPick: (Int) -> Void
    let onList: (() -> Void)?
    let profileShown: Bool?
    let onProfile: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            WaypointStrip(rows: rows, altitudes: altitudes, conflicts: conflicts, selected: selected, onPick: onPick)
                .frame(maxWidth: .infinity, alignment: .leading)
            if let onList {
                StripIconButton(icon: .list, label: "Show the plan as a list", action: onList)
            }
            if let shown = profileShown {
                StripIconButton(icon: .planTerrain, label: shown ? "Hide the terrain profile" : "Show the terrain profile", tint: shown ? theme.colors.primary : theme.colors.onSurfaceVariant, action: onProfile)
            }
        }
        .padding(.bottom, 8)
    }
}

struct EmptyMissionStrip: View {
    let homeSet: Bool
    let onTemplates: (() -> Void)?
    let onDownload: (() -> Void)?
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(TAP_TO_ADD).font(.titleMedium)
            if !homeSet {
                Text(FIRST_TAP_SETS_HOME).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if onTemplates != nil || onDownload != nil {
                HStack(spacing: Space.s4) {
                    if let onTemplates {
                        Button("Templates", action: onTemplates).buttonStyle(.borderless)
                    }
                    if let onDownload {
                        Button("Download from vehicle", action: onDownload).buttonStyle(.borderless)
                    }
                }
                .font(.labelLarge)
                .frame(minHeight: 40)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 12)
        .padding(.bottom, 4)
    }
}

struct SelectionHeader: View {
    let title: String
    let detail: String?
    let warning: Bool
    let open: Bool
    let onTitle: () -> Void
    let onDone: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            Button(action: onTitle) {
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: Space.s1) {
                        Text(title).font(.titleLarge).lineLimit(1).truncationMode(.tail)
                        Image(open ? .arrowDropDown : .arrowUp).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    if let detail {
                        Text(detail)
                            .font(.bodyMedium)
                            .foregroundStyle(warning ? theme.colors.error : theme.colors.onSurfaceVariant)
                            .lineLimit(1)
                            .truncationMode(.tail)
                    }
                }
                .foregroundStyle(theme.colors.onSurface)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.leading, 12)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityHint(open ? "Fold the settings" : "Show the settings")
            Button(action: onDone) {
                Label("Done", systemImage: Icon.check.rawValue)
            }
            .buttonStyle(.filled)
            .padding(.trailing, 4)
        }
        .padding(.bottom, 8)
    }
}
