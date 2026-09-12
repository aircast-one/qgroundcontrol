import Foundation

struct PatternGeometry: Equatable {
    let shape: String
    let vertices: [GeoPoint]
    let transects: [GeoPoint]
    let flightLoop: [GeoPoint]

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let shape = json["shape"] as? String, !shape.isEmpty else { return nil }
        self.shape = shape
        vertices = ((json["vertices"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        transects = ((json["transects"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        flightLoop = ((json["flightLoop"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
    }

    static func all(_ read: [String: Any]) -> [PatternGeometry] {
        ((read["items"] as? [[String: Any]]) ?? []).compactMap { PatternGeometry($0["geometry"]) }
    }

    var flownPath: [GeoPoint] {
        guard transects.isEmpty else { return transects }
        guard let start = flightLoop.first, flightLoop.count >= 3 else { return flightLoop }
        return flightLoop + [start]
    }

    static func flownLines(_ geometries: [PatternGeometry]) -> [[GeoPoint]] {
        geometries.map(\.flownPath).filter { $0.count >= 2 }
    }

    private static func shaped(_ geometries: [PatternGeometry], _ shape: String) -> [[GeoPoint]] {
        geometries.filter { $0.shape == shape }.map(\.vertices)
    }

    static func areas(_ geometries: [PatternGeometry]) -> [[GeoPoint]] {
        shaped(geometries, areaShape).filter { $0.count >= 3 }
    }

    static func lines(_ geometries: [PatternGeometry]) -> [[GeoPoint]] {
        shaped(geometries, lineShape).filter { $0.count >= 2 }
    }

    static let areaShape = "area"
    static let lineShape = "line"

    static let view = "view.missionItems(geometry)"
}
