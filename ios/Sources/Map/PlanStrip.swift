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
    let action: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: action) {
            Image(icon).frame(width: 48, height: 48).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .foregroundStyle(theme.colors.onSurface)
        .accessibilityLabel(label)
    }
}

struct WaypointStripBar: View {
    let rows: [ItemRow]
    let selected: Int?
    let onPick: (Int) -> Void
    let onList: (() -> Void)?
    let profileShown: Bool?
    let onProfile: () -> Void

    var body: some View {
        HStack(spacing: 0) {
            WaypointStrip(rows: rows, selected: selected, onPick: onPick)
                .frame(maxWidth: .infinity, alignment: .leading)
            if let onList {
                StripIconButton(icon: .list, label: "Show the plan as a list", action: onList)
            }
            if let shown = profileShown {
                StripIconButton(icon: shown ? .arrowDropDown : .arrowUp, label: shown ? "Hide the terrain profile" : "Show the terrain profile", action: onProfile)
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
    let onDone: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.titleLarge).lineLimit(1).truncationMode(.tail)
                if let detail {
                    Text(detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(2)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.leading, 12)
            Button(action: onDone) {
                Label("Done", systemImage: Icon.check.rawValue)
            }
            .buttonStyle(.filled)
            .padding(.trailing, 4)
        }
        .padding(.bottom, 8)
    }
}
