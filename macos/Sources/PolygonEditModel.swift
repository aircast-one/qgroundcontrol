import Foundation

struct EditablePolygon: Equatable {
    let path: String
    let points: [GeoPoint]
    let midpoints: [GeoPoint]
    let minimumVertices: Int
    let ring: Bool
    let closed: Bool
    let canRemoveVertex: Bool
    let segments: Int
    let splitInvokable: String
    let adjustInvokable: String
    let removeInvokable: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let path = json["path"] as? String, !path.isEmpty else { return nil }
        self.path = path
        points = ((json["vertices"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        midpoints = ((json["midpoints"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        minimumVertices = (json["minimumVertices"] as? NSNumber)?.intValue ?? 3
        ring = (json["ring"] as? NSNumber)?.boolValue ?? true
        closed = (json["closed"] as? NSNumber)?.boolValue ?? false
        canRemoveVertex = (json["canRemoveVertex"] as? NSNumber)?.boolValue ?? false
        segments = (json["segments"] as? NSNumber)?.intValue ?? 0
        splitInvokable = (json["splitInvokable"] as? String) ?? ""
        adjustInvokable = (json["adjustInvokable"] as? String) ?? ""
        removeInvokable = (json["removeInvokable"] as? String) ?? ""
    }
}

enum PolygonEdit {
    static func removes(_ index: Int, in polygon: EditablePolygon) -> Bool {
        polygon.canRemoveVertex && polygon.points.indices.contains(index)
    }

    static func splits(_ index: Int, in polygon: EditablePolygon) -> Bool {
        index >= 0 && index < polygon.segments
    }
}
