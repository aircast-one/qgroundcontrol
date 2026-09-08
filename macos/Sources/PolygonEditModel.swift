import Foundation

struct EditablePolygon: Equatable {
    let path: String
    let points: [GeoPoint]
    let minimumVertices: Int
    let ring: Bool

    static let defaultMinimum = 3
    static let ringSplit = "splitPolygonSegment"
    static let lineSplit = "splitSegment"

    var canRemoveVertex: Bool { points.count > minimumVertices }

    var closed: Bool { points.count >= minimumVertices }

    var splitInvokable: String { ring ? EditablePolygon.ringSplit : EditablePolygon.lineSplit }

    var segments: Int {
        guard closed else { return 0 }
        return ring ? points.count : points.count - 1
    }

    func midpoints() -> [GeoPoint] {
        guard segments > 0 else { return [] }
        return (0..<segments).map { index in
            let here = points[index]
            let next = points[(index + 1) % points.count]
            return GeoPoint(latitude: (here.latitude + next.latitude) / 2,
                            longitude: (here.longitude + next.longitude) / 2)
        }
    }

    static func read(path: String, json: [String: Any], ring: Bool) -> EditablePolygon? {
        let points = ((json["path"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        let minimum = (json["minVertexCount"] as? NSNumber)?.intValue
            ?? (ring ? defaultMinimum : 2)
        guard points.count >= minimum else { return nil }
        return EditablePolygon(path: path, points: points,
                               minimumVertices: minimum, ring: ring)
    }
}

enum PolygonEdit {
    static func adjust(_ index: Int, in polygon: EditablePolygon) -> String? {
        guard polygon.points.indices.contains(index) else { return nil }
        return "\(polygon.path).adjustVertex"
    }

    static func removes(_ index: Int, in polygon: EditablePolygon) -> Bool {
        polygon.canRemoveVertex && polygon.points.indices.contains(index)
    }

    static func splits(_ index: Int, in polygon: EditablePolygon) -> Bool {
        index >= 0 && index < polygon.segments
    }
}
