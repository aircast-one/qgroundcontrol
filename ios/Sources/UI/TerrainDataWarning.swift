import SwiftUI

let TERRAIN_DOWNLOAD = "view.terrainDownload"
let TERRAIN_STALL_MS = 30_000
let TERRAIN_STALLED_TITLE = "Terrain data isn't loading"
let TERRAIN_STALLED_DETAIL = "Terrain following may not work. It needs internet and terrain coverage for this area."
private let TERRAIN_DATA_WARNING_MAX_WIDTH: CGFloat = 420
private let TERRAIN_DATA_WARNING_CORNER: CGFloat = 12

struct TerrainLoad: Equatable {
    let loaded: Int64
    let pending: Int64
    var waiting: Bool { pending > 0 }
}

func terrainLoad(_ view: JSON?) -> TerrainLoad {
    TerrainLoad(loaded: view?["loaded"].int64 ?? 0, pending: view?["pending"].int64 ?? 0)
}

private struct TerrainProgressKey: Equatable {
    let waiting: Bool
    let loaded: Int64
}

struct TerrainDataWarning: View {
    @QgcPath(TERRAIN_DOWNLOAD) private var view
    @State private var stalled = false
    @State private var dismissed = false
    @Environment(\.theme) private var theme

    var body: some View {
        let load = terrainLoad(view)
        ZStack {
            Color.clear.invisibleAnchor()
            if stalled && !dismissed {
                HStack(spacing: Space.s3) {
                    Image(.warning)
                        .resizable()
                        .scaledToFit()
                        .foregroundStyle(theme.aircast.warning)
                        .frame(width: 24, height: 24)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(TERRAIN_STALLED_TITLE).font(.titleSmall)
                        Text(TERRAIN_STALLED_DETAIL).font(.bodySmall)
                    }
                    Button { dismissed = true } label: {
                        Image(.close).font(.system(size: 16)).frame(width: 32, height: 32).minimumTouchTarget()
                    }
                    .accessibilityLabel("Dismiss")
                }
                .padding(.leading, Space.s3)
                .padding(.vertical, Space.s2)
                .padding(.trailing, Space.s1)
                .foregroundStyle(theme.colors.onSurface)
                .background(theme.aircast.warningContainer, in: RoundedRectangle(cornerRadius: TERRAIN_DATA_WARNING_CORNER))
                .frame(maxWidth: TERRAIN_DATA_WARNING_MAX_WIDTH, alignment: .leading)
            }
        }
        .task(id: TerrainProgressKey(waiting: load.waiting, loaded: load.loaded)) {
            stalled = false
            if !load.waiting { dismissed = false }
            guard load.waiting else { return }
            try? await Task.sleep(for: .milliseconds(TERRAIN_STALL_MS))
            if !Task.isCancelled { stalled = true }
        }
    }
}
