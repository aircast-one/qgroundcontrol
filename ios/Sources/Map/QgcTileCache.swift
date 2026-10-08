import Foundation

struct TileProvider: Equatable {
    var prefix: String
    var type: String
    var count: Int
    var format: String
}

func tileHash(_ prefix: String, _ z: Int, _ x: Int, _ y: Int) -> String {
    prefix + String(format: "%08d%08d%03d", x, y, z)
}
