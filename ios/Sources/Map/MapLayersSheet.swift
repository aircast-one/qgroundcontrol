import SwiftUI

private let CHIP_ICON_SIZE: CGFloat = 12
private let CHIP_HEIGHT: CGFloat = 32
private let MIN_TOUCH_TARGET: CGFloat = 48
private let OBSTACLE_OVERLAY = "settings.flyViewSettings.showObstacleDistanceOverlay"

struct MapLayersSheet: View {
    let onDismiss: () -> Void
    @State private var listed: MapTypes?
    @MapPath(OBSTACLE_OVERLAY) private var obstacleJson
    @Environment(\.theme) private var theme

    private var obstacles: Bool { obstacleJson?["value"].bool == true }

    var body: some View {
        let types = listed
        AircastSheet(onDismissRequest: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s3) {
                VStack(alignment: .leading, spacing: 0) {
                    Text("Map layers").font(.headlineSmall)
                    Text("What the map shows while flying")
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                }
                PlanFlowRow(spacing: Space.s2, lineSpacing: 0) {
                    ForEach(types?.types ?? [], id: \.self) { type in
                        MapTypeChip(type: type, selected: type == types?.current) { choose(type) }
                    }
                }
                Button {
                    let shown = !obstacles
                    offMain { _ = setOk(OBSTACLE_OVERLAY, shown) }
                } label: {
                    HStack {
                        VStack(alignment: .leading, spacing: 0) {
                            Text("Obstacle distance").font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                            Text("Proximity sensor readings around the vehicle")
                                .font(.bodyMedium)
                                .foregroundStyle(theme.colors.onSurfaceVariant)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        Toggle("", isOn: .constant(obstacles))
                            .labelsHidden()
                            .allowsHitTesting(false)
                    }
                    .padding(.vertical, Space.s2)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, Space.s6)
            .padding(.bottom, Space.s8)
        }
        .task { listed = await offMain { mapTypes(MapBridge.read(MAP_TYPES_VIEW)) } }
    }

    private func choose(_ type: String) {
        guard let path = listed?.path else { return }
        Task {
            _ = await offMain { setOk(path, type) }
            listed = listed.map { MapTypes(current: type, types: $0.types, path: $0.path) }
        }
    }
}

private struct MapTypeChip: View {
    let type: String
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack(spacing: Space.s1) {
                if selected { Image(.check).font(.system(size: CHIP_ICON_SIZE, weight: .semibold)) }
                Text(type).font(.labelLarge).lineLimit(1)
            }
            .padding(.horizontal, Space.s3)
            .frame(height: CHIP_HEIGHT)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(selected ? theme.colors.secondaryContainer : .clear, in: RoundedRectangle(cornerRadius: Corner.small))
            .overlay(RoundedRectangle(cornerRadius: Corner.small).stroke(selected ? .clear : theme.colors.outline, lineWidth: 1))
            .frame(height: MIN_TOUCH_TARGET)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}
