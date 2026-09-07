import Foundation

struct TileAddress: Equatable {
    let x: Int
    let y: Int
    let z: Int
}

struct TileCrop: Equatable {
    let x: Double
    let y: Double
    let size: Double
}

enum TilePyramid {
    static let maximumParentDepth = 4

    static func parent(of tile: TileAddress, depth: Int) -> TileAddress? {
        guard depth > 0, tile.z - depth >= 0 else { return nil }
        return TileAddress(x: tile.x >> depth, y: tile.y >> depth, z: tile.z - depth)
    }

    static func crop(of tile: TileAddress, within parent: TileAddress) -> TileCrop? {
        let depth = tile.z - parent.z
        guard depth > 0, depth <= 24, TilePyramid.parent(of: tile, depth: depth) == parent else {
            return nil
        }
        let span = Double(1 << depth)
        return TileCrop(x: Double(tile.x - (parent.x << depth)) / span,
                        y: Double(tile.y - (parent.y << depth)) / span,
                        size: 1 / span)
    }

    static func children(of tile: TileAddress) -> [TileAddress] {
        [(0, 0), (1, 0), (0, 1), (1, 1)].map {
            TileAddress(x: tile.x * 2 + $0.0, y: tile.y * 2 + $0.1, z: tile.z + 1)
        }
    }

    static func quadrant(of child: TileAddress) -> (column: Int, row: Int) {
        (child.x - (child.x >> 1) * 2, child.y - (child.y >> 1) * 2)
    }
}
