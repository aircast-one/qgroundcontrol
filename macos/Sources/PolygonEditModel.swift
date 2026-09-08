import Foundation

struct EditablePolygon: Equatable {
    let path: String
    let points: [GeoPoint]
    let minimumVertices: Int

    static let defaultMinimum = 3

    var canRemoveVertex: Bool { points.count > minimumVertices }

    var closed: Bool { points.count >= minimumVertices }

    func midpoints() -> [GeoPoint] {
        guard closed else { return [] }
        return points.indices.map { index in
            let next = points[(index + 1) % points.count]
            let here = points[index]
            return GeoPoint(latitude: (here.latitude + next.latitude) / 2,
                            longitude: (here.longitude + next.longitude) / 2)
        }
    }

    static func read(path: String, json: [String: Any]) -> EditablePolygon? {
        let points = ((json["path"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        guard points.count >= defaultMinimum else { return nil }
        let minimum = (json["minVertexCount"] as? NSNumber)?.intValue ?? defaultMinimum
        return EditablePolygon(path: path, points: points, minimumVertices: minimum)
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
        polygon.closed && polygon.points.indices.contains(index)
    }
}
