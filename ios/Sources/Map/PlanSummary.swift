import Foundation

func planSummary(
    _ itemCount: Int,
    _ shape: [String],
    _ items: [MissionItem],
    _ polygons: [FencePolygon],
    _ circles: [FenceCircle],
    _ rally: [RallyPoint],
    _ surveys: [Survey],
    _ summaryText: String,
    _ selected: MapHit?,
    offline: Bool = true,
    canAddByHand: Bool = true,
    canPlaceByButton: Bool = true
) -> String {
    let fences = polygons.count + circles.count
    let scanPoints = surveys.map(\.transects.count).reduce(0, +)
    let counts = [
        itemCount > 0 ? "\(itemCount) item\(itemCount == 1 ? "" : "s")" + (shape.isEmpty ? "" : " (\(shape.joined(separator: ", ")))") : nil,
        fences > 0 ? "\(fences) fence\(fences == 1 ? "" : "s")" : nil,
        rally.isEmpty ? nil : "\(rally.count) rally",
        scanPoints > 0 ? "\(scanPoints) scan pts" : nil,
    ].compactMap { $0 }
    guard !counts.isEmpty else {
        let add = canAddByHand ? "long press to add"
            : canPlaceByButton ? "add a takeoff to start"
            : "move the map to where you will fly, then add a takeoff"
        return offline ? "Empty plan · \(add)" : "Empty plan · Download the aircraft's, or \(add)"
    }
    return (counts + [summaryText.isEmpty ? nil : summaryText, selectionText(selected, items, circles, polygons)].compactMap { $0 })
        .joined(separator: " · ")
}

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
