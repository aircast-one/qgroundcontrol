import Foundation

let NO_POSITION = "no position"
let AFTER_THE_ROUTE_ENDS = "after the route ends"

func itemCountText(_ count: Int) -> String { count == 1 ? "1 item" : "\(count) items" }

struct ItemRow: Equatable, Identifiable {
    var index: Int
    var number: String
    var name: String
    var detail: String
    var colour: String
    var placed: Bool
    var readyForSave: Bool = true

    var id: Int { index }
}

let NOT_READY_SEAL = "?"

func altitudeWithFrame(_ item: MissionItem) -> String? {
    guard let height = [item.altitudeText, item.altitudeBandText].first(where: { !$0.isBlank }) else { return nil }
    return item.altitudeFrameText.isBlank ? height : "\(height) \(item.altitudeFrameText)"
}

func altitudeFieldLabel(_ item: MissionItem) -> String {
    let unit = item.altitudeEditUnits.ifBlank("m")
    return item.altitudeFrameText.isBlank ? "Alt \(unit)" : "Alt \(unit) \(item.altitudeFrameText)"
}

func rallyAltitudeLabel(_ point: RallyPoint) -> String { "Alt \(point.altitudeUnits.ifBlank("m"))" }

func rallyAltitudeIsEditable(_ point: RallyPoint) -> Bool { !point.altitude.isNaN && !point.altitudePath.isBlank }

func sequenceLabel(_ item: MissionItem) -> String {
    item.foldedCommands > 0 ? "\(item.sequence)\u{2013}\(item.sequence + item.foldedCommands)" : "\(item.sequence)"
}

func itemPlace(_ item: MissionItem, _ items: [MissionItem]) -> String? {
    let listed = items.filter { $0.index != HOME_ITEM }
    guard let at = listed.firstIndex(where: { $0.index == item.index }) else { return nil }
    let previous = items.last { $0.index < item.index }
    let leg = previous.flatMap { before in
        !item.distance.isNaN && item.distance > 0 && !item.distanceText.isBlank ? "\(item.distanceText) from item \(sequenceLabel(before))" : nil
    }
    return ["Item \(at + 1) of \(listed.count)", leg].compactMap { $0 }.joined(separator: " \u{00b7} ")
}

func surveyTiles(_ item: MissionItem, _ stats: SurveyStats?) -> [(String, String)] {
    guard let stats else { return [] }
    return [
        ("AREA", stats.areaText),
        ("DISTANCE", item.kind != KIND_STRUCTURE ? stats.distanceText : ""),
        ("PHOTOS", item.cameraShots > 0 ? "\(item.cameraShots)" : ""),
        ("INTERVAL", stats.intervalText),
    ].filter { !$0.1.isBlank }
}

func sheetDetail(_ item: MissionItem, _ stats: SurveyStats?) -> String {
    guard !surveyTiles(item, stats).isEmpty else { return itemDetail(item, stats) }
    var bare = item
    bare.cameraShots = 0
    return itemDetail(bare, stats.map { var cleared = $0; cleared.areaText = ""; return cleared })
}

func deleteLabel(_ item: MissionItem) -> String { "Delete \(item.command.ifBlank("item").lowercased())" }

func itemRows(_ items: [MissionItem], _ stats: [Int: SurveyStats] = [:]) -> [ItemRow] {
    items.map { item in
        ItemRow(
            index: item.index,
            number: item.readyForSave ? sequenceLabel(item) : NOT_READY_SEAL,
            name: item.command.ifBlank("Item \(item.sequence)"),
            detail: itemDetail(item, stats[item.index]),
            colour: waypointColour(item.kind, item.commandId),
            placed: item.placed,
            readyForSave: item.readyForSave
        )
    }
}

private func nonBlank(_ text: String?) -> String? { text.flatMap { $0.isBlank ? nil : $0 } }

func itemDetail(_ item: MissionItem, _ stats: SurveyStats? = nil) -> String {
    [
        altitudeWithFrame(item) ?? (!item.placed && item.specifiesCoordinate ? NO_POSITION : nil),
        item.afterRouteEnds ? AFTER_THE_ROUTE_ENDS : nil,
        nonBlank(item.speedChangeText),
        nonBlank(stats?.areaText),
        photosText(item.cameraShots),
        holdText(item.extraSeconds),
        nonBlank(item.blockedReason),
        nonBlank(stats?.warning),
    ].compactMap { $0 }.joined(separator: " \u{00b7} ")
}

func worthListing(_ items: [MissionItem]) -> Bool { items.contains { $0.index != HOME_ITEM } }

func rowAt(_ rows: [ItemRow], _ index: Int) -> ItemRow? { rows.first { $0.index == index } }

func selectionAfterRemove(_ removed: Int, _ countBefore: Int) -> MapHit? {
    countBefore - 2 > HOME_ITEM ? .Waypoint(index: min(removed, countBefore - 2)) : nil
}

func missionItemIndex(_ selected: MapHit?) -> Int? {
    switch selected {
    case .Waypoint(let index), .LandingPlace(let index, _), .LoiterRadius(let index), .LoiterRotation(let index): index
    case .SurveyVertex(let item, _): item
    case .ShapeCentre(let fence, let owner), .ShapeRadius(let fence, let owner): fence ? nil : owner
    default: nil
    }
}

func insertAfter(_ selected: MapHit?, _ items: [MissionItem]) -> Int {
    guard let index = missionItemIndex(selected), items.contains(where: { $0.index == index }) else { return AT_END }
    return index + 1 >= items.count ? AT_END : index + 1
}

func selectionSequence(_ selected: MapHit?, _ items: [MissionItem]) -> Int? {
    missionItemIndex(selected).flatMap { index in items.first { $0.index == index }?.sequence }
}

func addingAfterText(_ selected: MapHit?, _ items: [MissionItem]) -> String? {
    missionItemIndex(selected).flatMap { index in items.first { $0.index == index } }.map { "Adding after #\($0.sequence)" }
}

func legText(_ item: MissionItem) -> String? {
    let parts = [
        ("Alt diff", item.altitudeChangeText),
        ("Azimuth", item.azimuthText),
        ("Heading", item.headingText),
        ("Gradient", item.gradientText),
        ("Prev WP", item.distanceText),
    ].filter { !$0.1.isBlank }
    return parts.isEmpty ? nil : parts.map { "\($0.0) \($0.1)" }.joined(separator: " \u{00b7} ")
}

func movedText(_ hit: MapHit, _ items: [MissionItem]) -> String {
    switch hit {
    case .Waypoint(let index): items.first { $0.index == index }.map { "Moved #\($0.sequence)" } ?? "Moved an item"
    case .FenceVertex: "Moved a fence corner"
    case .SurveyVertex: "Moved a survey corner"
    case .Rally: "Moved a rally point"
    case .BreachReturn: "Moved the breach return point"
    case .CircleCentre: "Moved a fence circle"
    case .CircleRadius: "Changed a fence radius"
    case .ShapeCentre(let fence, _): fence ? "Moved a fence" : "Moved a survey area"
    case .ShapeRadius: "Changed the circle radius"
    case .LoiterRadius: "Changed the loiter radius"
    case .LoiterRotation: "Changed the loiter direction"
    case .Circle: "Changed a fence radius"
    case .Midpoint: "Added a corner"
    case .LandingPlace(_, let place): place == LANDING_PLACE_APPROACH ? "Moved the final approach" : "Moved the touchdown"
    }
}

private let moveWrites = NSRecursiveLock()
private let movesCounted = NSLock()
private var committedMoves: Int64 = 0

func moveGeneration() -> Int64 { movesCounted.withLock { committedMoves } }

func writeDragStep(
    _ generation: Int64,
    _ hit: MapHit,
    _ latitude: Double,
    _ longitude: Double,
    _ surveys: [Survey],
    _ rally: [RallyPoint],
    _ fences: [FencePolygon],
    _ items: [MissionItem],
    _ circles: [FenceCircle] = []
) -> Bool {
    guard moveWrites.try() else { return true }
    defer { moveWrites.unlock() }
    return generation != moveGeneration() || applyMove(hit, latitude, longitude, surveys, rally, fences, items, circles)
}

func writeMove(
    _ hit: MapHit,
    _ latitude: Double,
    _ longitude: Double,
    _ surveys: [Survey],
    _ rally: [RallyPoint],
    _ fences: [FencePolygon] = [],
    _ items: [MissionItem] = [],
    _ circles: [FenceCircle] = []
) -> Bool {
    moveWrites.withLock {
        movesCounted.withLock { committedMoves += 1 }
        return applyMove(hit, latitude, longitude, surveys, rally, fences, items, circles)
    }
}

private func applyMove(
    _ hit: MapHit,
    _ latitude: Double,
    _ longitude: Double,
    _ surveys: [Survey],
    _ rally: [RallyPoint],
    _ fences: [FencePolygon],
    _ items: [MissionItem],
    _ circles: [FenceCircle]
) -> Bool {
    let to = TrackPoint(latitude: latitude, longitude: longitude)
    func target(_ fence: Bool, _ owner: Int) -> ShapeTarget? {
        fence ? shapeTarget(owner, nil) : shapeTarget(nil, surveys.first { $0.index == owner })
    }
    switch hit {
    case .Waypoint(let index):
        return PlanBridge.moveItem(index, latitude, longitude)
    case .FenceVertex(let polygon, let vertex):
        return FenceBridge.adjustVertex(polygon, vertex, latitude, longitude)
    case .SurveyVertex(let item, let vertex):
        return surveys.first { $0.index == item }.map { SurveyBridge.adjustVertex($0, vertex, latitude, longitude) } == true
    case .Rally(let index):
        return FenceBridge.moveRallyPoint(index, latitude, longitude, rallyAltitudeFor(rally, index))
    case .CircleCentre(let index):
        return FenceBridge.moveCircle(index, latitude, longitude)
    case .CircleRadius(let index):
        return circles.first { $0.index == index }.map { FenceBridge.setCircleRadius($0.index, draggedCircleRadius($0, to)) } == true
    case .ShapeCentre(let fence, let owner):
        let vertices = fence ? fences.first { $0.index == owner }?.vertices : surveys.first { $0.index == owner }?.area
        guard let shape = target(fence, owner), let moved = vertices.flatMap({ shapeMovedTo($0, to) }) else { return false }
        return replaceShape(shape, moved)
    case .ShapeRadius(let fence, let owner):
        guard let shape = target(fence, owner) else { return false }
        let current = shapeVertices(shape, fences, surveys)
        guard let ring = polygonCentre(current).flatMap({ circleAround(current, metresBetween($0, to)) }) else { return false }
        return replaceShape(shape, ring)
    case .LoiterRadius(let index):
        return items.first { $0.index == index }.map { PlanBridge.setLoiterRadius($0.index, draggedLoiterRadius($0, to)) } == true
    case .LoiterRotation:
        return false
    case .BreachReturn:
        return FenceBridge.setBreachReturn(to)
    case .Circle:
        return true
    case .Midpoint:
        return false
    case .LandingPlace(let index, let place):
        return moveLandingPlace(index, place, latitude, longitude)
    }
}

func patternName(_ index: Int, _ items: [MissionItem]) -> String {
    items.first { $0.index == index }.flatMap { $0.command.isBlank ? nil : $0.command } ?? "pattern"
}

func layersText(_ survey: Survey?) -> String? {
    guard let survey, survey.layers > 1 else { return nil }
    return survey.layerSpanText.isBlank ? "\(survey.layers) layers, one drawn" : "\(survey.layers) layers, \(survey.layerSpanText), one drawn"
}

func cameraText(_ stats: SurveyStats?) -> String? {
    let parts = [
        nonBlank(stats?.surfaceDistanceText).map { "\($0) above the surface" },
        nonBlank(stats?.footprintText).map { "each shot covers \($0)" },
        nonBlank(stats?.intervalText).map { "a shot every \($0)" },
    ].compactMap { $0 }
    return parts.isEmpty ? nil : parts.joined(separator: " \u{00b7} ")
}

func photosText(_ shots: Int) -> String? {
    shots <= 0 ? nil : shots == 1 ? "1 photo" : "\(shots) photos"
}

func holdText(_ seconds: Double) -> String? {
    seconds.isNaN || seconds <= 0 ? nil : "holds \(Int(min(seconds, Double(Int32.max)))) s"
}
