import XCTest
@testable import Aircast

final class LoiterHandleTests: XCTestCase {
    private let loiter = MissionItem(index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: "", selected: false, loiterRadius: 80.0)
    private let waypoint = MissionItem(index: 1, sequence: 1, latitude: 47.1, longitude: 8.0, command: "", selected: false)

    func testTheArrowsRunWithTheTurnTheWayQgcMapCircleVisualsPointsThem() {
        XCTAssertEqual(loiterRotationArrows([waypoint, loiter]).map(\.bearing), [90.0, 270.0])
        XCTAssertEqual(loiterRotationArrows([withChanges(loiter) { $0.loiterRadius = -80.0 }]).map(\.bearing), [270.0, 90.0], "counter-clockwise flips them")
    }

    func testOnlyTheCurrentLoiterCanBeDraggedOrFlippedEveryLoiterShowsItsArrows() {
        let kinds = { (selected: Int?) in loiterHandleFeatures([self.waypoint, self.loiter], selected).map { $0.getStringProperty(HANDLE_KIND_PROPERTY) } }
        XCTAssertEqual(kinds(nil), [], "QGCMapCircleVisuals: the arrows' mouse area is visible only while interactive")
        XCTAssertEqual(loiterRotationArrows([waypoint, loiter]).count, 2)
        XCTAssertEqual(kinds(2), [HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_RADIUS])
        XCTAssertEqual(handleHit(HANDLE_KIND_LOITER_RADIUS, 2, 0), .LoiterRadius(index: 2))
        XCTAssertEqual(handleHit(HANDLE_KIND_LOITER_ROTATION, 2, 1), .LoiterRotation(index: 2))
    }

    func testDraggingTheRadiusKeepsTheDirectionItTurns() {
        let to = pointAt(TrackPoint(latitude: 47.0, longitude: 8.0), 150.0, 90.0)
        XCTAssertEqual(draggedLoiterRadius(loiter, to), 150.0, accuracy: 0.5)
        XCTAssertEqual(draggedLoiterRadius(withChanges(loiter) { $0.loiterRadius = -80.0 }, to), -150.0, accuracy: 0.5)
    }
}

final class ClickMarkerTests: XCTestCase {
    func testTheMarkerSitsOnTheTappedPointAndNowhereOnceTheMenuCloses() {
        XCTAssertEqual(clickMarkerFeatures(TrackPoint(latitude: 47.0, longitude: 8.0)).shapes.count, 1)
        XCTAssertEqual(clickMarkerFeatures(nil).shapes.count, 0)
        XCTAssertEqual(clickMarkerFeatures(TrackPoint(latitude: .nan, longitude: 8.0)).shapes.count, 0)
    }
}

final class CollisionLegTests: XCTestCase {
    func testLegsTheCoreSaysHitTerrainAreDrawnRedAndNothingElse() {
        let view = JSON.parse(#"{"collisionLegs":[{"from":{"latitude":47.0,"longitude":8.0},"to":{"latitude":47.01,"longitude":8.0}},{"from":{"latitude":0,"longitude":0},"to":{"latitude":47.0,"longitude":8.0}}]}"#)
        let legs = collisionLegs(view)
        XCTAssertEqual(legs.map(\.0), [TrackPoint(latitude: 47.0, longitude: 8.0)])
        XCTAssertEqual(legs.map(\.1), [TrackPoint(latitude: 47.01, longitude: 8.0)])
        XCTAssertEqual(collisionLegFeatures(legs).shapes.count, 1)
        XCTAssertTrue(collisionLegs(nil).isEmpty)
    }
}

final class MarkerLabelTests: XCTestCase {
    func testALetteredItemShowsItsFirstLetterAndItsNameBesideItLikeMissionItemIndicator() {
        XCTAssertEqual(waypointLabel(1, false, abbreviation: "Takeoff"), "T")
        XCTAssertEqual(sideLabel(false, "Takeoff"), "Takeoff")
        XCTAssertEqual(waypointLabel(4, false, abbreviation: ""), "4")
        XCTAssertEqual(sideLabel(false, ""), "")
        XCTAssertEqual(waypointLabel(5, false, abbreviation: "S"), "S")
        XCTAssertEqual(sideLabel(false, "S"), "", "a one-letter abbreviation sits only in the circle")
        XCTAssertEqual(sideLabel(true, "Loiter"), "")
    }
}
