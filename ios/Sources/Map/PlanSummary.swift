import Foundation

func circleText(_ circle: FenceCircle) -> String { circle.detailText }

func selectionText(
    _ selected: MapHit?,
    _ items: [MissionItem],
    _ circles: [FenceCircle],
    _ polygons: [FencePolygon]
) -> String? {
    switch selected {
    case .Waypoint(let index):
        items.first { $0.index == index }.flatMap { item in altitudeWithFrame(item).map { "#\(item.sequence) at \($0)" } }
    case .Circle(let index), .CircleCentre(let index), .CircleRadius(let index):
        circles.first { $0.index == index }.map(circleText)
    case .FenceVertex(let polygon, let vertex):
        polygons.first { $0.index == polygon }.map { "corner \(vertex + 1) of \($0.vertices.count)" }
    case .LandingPlace(_, let place):
        place == LANDING_PLACE_APPROACH ? "final approach" : "touchdown"
    case .SurveyVertex, .Midpoint, .ShapeCentre, .ShapeRadius, .LoiterRadius, .LoiterRotation, .Rally, .BreachReturn, nil:
        nil
    }
}
