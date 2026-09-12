import Foundation

struct PatternGeometry: Equatable {
    let shape: String
    let vertices: [GeoPoint]
    let transects: [GeoPoint]

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let shape = json["shape"] as? String, !shape.isEmpty else { return nil }
        self.shape = shape
        vertices = ((json["vertices"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        transects = ((json["transects"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
    }

    static func all(_ read: [String: Any]) -> [PatternGeometry] {
        ((read["items"] as? [[String: Any]]) ?? []).compactMap { PatternGeometry($0["geometry"]) }
    }

    static func flownLines(_ geometries: [PatternGeometry]) -> [[GeoPoint]] {
        geometries.map(\.transects).filter { $0.count >= 2 }
    }

    static let view = "view.missionItems(geometry)"
}
