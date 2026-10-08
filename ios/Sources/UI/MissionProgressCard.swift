import SwiftUI

private let MISSION_PROGRESS = "view.missionProgress"
private let CARD_MAX_WIDTH: CGFloat = 360

struct MissionProgress: Equatable {
    var current: Int
    var last: Int
    var fraction: Double
    var distance: String
    var skipTo: Int?
}

func missionProgress(_ view: JSON?) -> MissionProgress? {
    guard let view, view["shown"].bool else { return nil }
    return MissionProgress(
        current: view["current"].int(0),
        last: view["last"].int(0),
        fraction: view["fraction"].double(0),
        distance: [view["distanceToNext"].string, view["distanceUnits"].string].filter { !$0.isBlank }.joined(separator: " "),
        skipTo: view["canSkip"].bool ? view["skipTo"].int(0) : nil
    )
}

func missionProgressLine(_ progress: MissionProgress) -> String {
    ["To waypoint \(progress.current) of \(progress.last)", progress.distance].filter { !$0.isBlank }.joined(separator: " \u{00B7} ")
}

struct MissionProgressCard: View {
    @QgcPath(MISSION_PROGRESS) private var view
    @State private var skipTarget: Int?
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        if let progress = missionProgress(view) {
            VStack(alignment: .leading, spacing: Space.s2) {
                HStack {
                    Text(missionProgressLine(progress))
                        .font(.labelLarge)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    if let next = progress.skipTo {
                        Button("Skip") { skipTarget = next }
                            .buttonStyle(.borderless)
                    }
                }
                ProgressView(value: min(max(progress.fraction, 0), 1))
            }
            .padding(Space.s3)
            .frame(maxWidth: CARD_MAX_WIDTH)
            .background(osdBackdrop(theme.colors.surfaceContainerHigh, flyOsd), in: RoundedRectangle(cornerRadius: Corner.medium))
            .overlay {
                if let next = skipTarget {
                    SetWaypointSheet(sequence: next, onDismiss: { skipTarget = nil })
                }
            }
        }
    }
}
