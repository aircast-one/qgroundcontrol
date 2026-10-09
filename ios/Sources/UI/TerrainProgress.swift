import SwiftUI

let TERRAIN_DOWNLOAD = "view.terrainDownload"
private let TERRAIN_HIDE_AFTER_MS = 15_000
private let TERRAIN_GREEN = Color(hex: 0x00C853)
private let BAR_WIDTH: CGFloat = 160
private let BAR_HEIGHT: CGFloat = 18

struct TerrainLoad: Equatable {
    let loaded: Int64
    let pending: Int64
    let fraction: Double
}

func terrainLoad(_ view: JSON?) -> TerrainLoad? {
    guard let view, view.has("loaded") else { return nil }
    return TerrainLoad(loaded: view["loaded"].int64 ?? 0, pending: view["pending"].int64 ?? 0, fraction: view["fraction"].double(0))
}

func terrainShowsNow(_ load: TerrainLoad) -> Bool { load.pending > 0 || load.loaded != 0 }

private struct TerrainCount: Equatable {
    let loaded: Int64
    let pending: Int64
}

struct TerrainProgress: View {
    @QgcPath(TERRAIN_DOWNLOAD) private var view
    @State private var visible = false
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        if let load = terrainLoad(view) {
            ZStack {
                Color.clear.invisibleAnchor()
                if visible {
                    VStack(spacing: 0) {
                        Text("Terrain load progress").font(.labelSmall)
                        ZStack(alignment: .leading) {
                            Rectangle().stroke(TERRAIN_GREEN, lineWidth: 1)
                            ZStack {
                                Rectangle().fill(TERRAIN_GREEN)
                                if load.pending == 0 {
                                    Text("Done").font(.labelSmall)
                                }
                            }
                            .frame(width: BAR_WIDTH * min(max(load.fraction, 0), 1))
                        }
                        .frame(width: BAR_WIDTH, height: BAR_HEIGHT)
                    }
                    .padding(6)
                    .background(osdBackdrop(theme.colors.surface.opacity(0.85), flyOsd), in: RoundedRectangle(cornerRadius: Corner.medium))
                    .contentShape(RoundedRectangle(cornerRadius: Corner.medium))
                }
            }
            .task(id: TerrainCount(loaded: load.loaded, pending: load.pending)) {
                if terrainShowsNow(load) { visible = true }
                guard load.pending == 0, visible else { return }
                try? await Task.sleep(for: .milliseconds(TERRAIN_HIDE_AFTER_MS))
                if !Task.isCancelled { visible = false }
            }
        }
    }
}
