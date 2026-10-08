import Foundation

func selectionSurvives(
    _ selected: MapHit?,
    _ items: [MissionItem],
    _ polygons: [FencePolygon],
    _ circles: [FenceCircle],
    _ rally: [RallyPoint],
    _ surveys: [Survey],
    landings: [LandingPattern] = [],
    breach: Bool = false
) -> Bool {
    switch selected {
    case nil: true
    case .Waypoint(let index): items.contains { $0.index == index }
    case .FenceVertex(let polygon, let vertex): polygons.contains { $0.index == polygon && $0.vertices.indices.contains(vertex) }
    case .SurveyVertex(let item, let vertex): surveys.contains { $0.index == item && $0.area.indices.contains(vertex) }
    case .Rally(let index): rally.contains { $0.index == index }
    case .BreachReturn: breach
    case .Circle(let index), .CircleCentre(let index), .CircleRadius(let index): circles.contains { $0.index == index }
    case .Midpoint: false
    case .LandingPlace(let index, _): landings.contains { $0.index == index }
    case .ShapeCentre(let fence, let owner), .ShapeRadius(let fence, let owner):
        fence ? polygons.contains { $0.index == owner } : surveys.contains { $0.index == owner }
    case .LoiterRadius, .LoiterRotation: false
    }
}

func selectedItem(_ selected: MapHit?) -> Int? {
    switch selected {
    case .Waypoint(let index), .LandingPlace(let index, _), .LoiterRadius(let index), .LoiterRotation(let index): index
    case .SurveyVertex(let item, _): item
    case .ShapeCentre(let fence, let owner), .ShapeRadius(let fence, let owner): fence ? nil : owner
    default: nil
    }
}

struct SelectedFence {
    var keepsIn: Bool
    var kindText: String
    var detailText: String
    var flip: (() -> Bool)?
}

private func fenceOf(_ polygon: FencePolygon) -> SelectedFence {
    SelectedFence(
        keepsIn: polygon.inclusion,
        kindText: polygon.kindText,
        detailText: polygon.detailText,
        flip: { FenceBridge.setPolygonInclusion(polygon.index, !polygon.inclusion) }
    )
}

private func fenceOf(_ circle: FenceCircle) -> SelectedFence {
    SelectedFence(keepsIn: circle.inclusion, kindText: circle.kindText, detailText: circle.detailText, flip: nil)
}

func selectedFence(_ selected: MapHit?, _ polygons: [FencePolygon], _ circles: [FenceCircle]) -> SelectedFence? {
    func circleAt(_ index: Int) -> SelectedFence? { circles.first { $0.index == index }.map(fenceOf) }
    switch selected {
    case .FenceVertex(let polygon, _): return polygons.first { $0.index == polygon }.map(fenceOf)
    case .Circle(let index), .CircleCentre(let index): return circleAt(index)
    default: return nil
    }
}

func fenceDetail(_ selected: MapHit?, _ polygons: [FencePolygon], _ circles: [FenceCircle]) -> String? {
    selectedFence(selected, polygons, circles)
        .map { [$0.kindText, inclusionText($0.keepsIn), $0.detailText].filter { !$0.isBlank } }
        .flatMap { $0.isEmpty ? nil : $0.joined(separator: " · ") }
}

func selectedLanding(_ selected: MapHit?, _ landings: [LandingPattern]) -> LandingPattern? {
    selectedItem(selected).flatMap { index in landings.first { $0.index == index } }
}

func selectedSurvey(_ selected: MapHit?, _ surveys: [Survey]) -> Survey? {
    selectedItem(selected).flatMap { index in surveys.first { $0.index == index } }
}
