import SwiftUI

struct MapLayersSheet: View {
    let onDismiss: () -> Void
    @State private var listed: MapTypes?
    @MapPath(OBSTACLE_OVERLAY) private var obstacleJson
    @Environment(\.theme) private var theme

    private var obstacles: Bool { obstacleJson?["value"].bool == true }

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s3) {
                VStack(alignment: .leading, spacing: 0) {
                    Text("Map layers").font(.headlineSmall)
                    Text("What the map shows while flying")
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                }
                PlanFlowRow(spacing: Space.s2, lineSpacing: Space.s2) {
                    ForEach(listed?.types ?? [], id: \.self) { type in
                        MapTypeChip(type: type, selected: type == listed?.current) { choose(type) }
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
            .padding(.top, Space.s4)
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
                if selected { Image(systemName: "checkmark").font(.system(size: 12, weight: .semibold)) }
                Text(type).font(.labelLarge).lineLimit(1)
            }
            .padding(.horizontal, Space.s3)
            .frame(height: 32)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(selected ? theme.colors.secondaryContainer : .clear, in: RoundedRectangle(cornerRadius: Corner.small))
            .overlay(RoundedRectangle(cornerRadius: Corner.small).stroke(selected ? .clear : theme.colors.outline, lineWidth: 1))
        }
        .buttonStyle(.plain)
    }
}

private let OBSTACLE_OVERLAY = "settings.flyViewSettings.showObstacleDistanceOverlay"
