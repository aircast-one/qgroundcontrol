import Foundation

struct EditableShape: Equatable {
    var path: String
    var midpoints: [TrackPoint]
    var splitInvokable: String
    var canRemoveVertex: Bool
    var edgeLengths: [String] = []
    var distanceUnit: String = "m"
    var metresPerUnit: Double = 1
    var caption: String = ""
    var circleCaption: String = ""
}

func editableShape(_ path: String, _ shape: String = "") -> EditableShape? {
    let call = shape.isBlank ? "view.polygon(\(path))" : "view.polygon(\(path),\(shape))"
    guard let view = MapBridge.read(call), view["class"].string == "EditablePolygon" else { return nil }
    let perUnit = view["horizontalMetresPerUnit"].double(1)
    return EditableShape(
        path: path,
        midpoints: view["midpoints"].array.compactMap { $0.object == nil ? nil : coordinate($0) },
        splitInvokable: view["splitInvokable"].string,
        canRemoveVertex: view["canRemoveVertex"].bool,
        edgeLengths: view["edgeLengths"].arrayOrNil?.map(\.string) ?? [],
        distanceUnit: view["horizontalUnit"].string.ifBlank("m"),
        metresPerUnit: perUnit.isFinite && perUnit > 0 ? perUnit : 1,
        caption: view["caption"].string,
        circleCaption: view["circleCaption"].string
    )
}

func edgeLabels(_ hit: MapHit?, _ fences: [FencePolygon], _ surveys: [Survey]) -> [LandingLabel] {
    let shape: EditableShape? = switch hit {
    case .FenceVertex(let polygon, _): fences.first { $0.index == polygon }?.editable
    case .SurveyVertex(let item, _): surveys.first { $0.index == item }?.editable
    default: nil
    }
    guard let shape else { return [] }
    return zip(shape.midpoints, shape.edgeLengths).map { at, text in LandingLabel(at: at, text: text) }
}
