import XCTest
@testable import Aircast

final class TransectMarksTests: XCTestCase {
    private func lawnmower(_ transects: Int, _ turnaround: Bool) -> [TrackPoint] {
        (0..<transects).flatMap { row -> [TrackPoint] in
            let latitude = 47.0 + Double(row) * 0.001
            let pair = [TrackPoint(latitude: latitude, longitude: 8.0), TrackPoint(latitude: latitude, longitude: 8.01)]
            let ends = row % 2 == 0 ? pair : pair.reversed()
            return turnaround ? [ends[0], ends[0], ends[1], ends[1]] : ends
        }
    }

    func testTheCurrentSurveyShowsEntryAndExitArrowsTwoEachOnceThereAreMoreThanThreeTransects() throws {
        XCTAssertEqual(transectMarks(lawnmower(3, false), false, true).arrows.count, 2)
        let many = transectMarks(lawnmower(5, false), false, true)
        XCTAssertEqual(many.arrows.count, 4)
        XCTAssertEqual(try XCTUnwrap(many.arrows.first).bearing, 90.0, accuracy: 1.0)
        XCTAssertTrue(many.stubs.isEmpty)
    }

    func testAnotherSurveyWithTurnaroundsShowsOnlyItsEntryAndExitStubs() {
        let marks = transectMarks(lawnmower(4, true), true, false)
        XCTAssertTrue(marks.arrows.isEmpty)
        XCTAssertEqual(marks.stubs.count, 2)
        XCTAssertTrue(transectMarks(lawnmower(4, false), false, false).stubs.isEmpty)
    }

    private func leg(_ index: Int, complex: Bool = false) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: 47.0 + Double(index) * 0.001, longitude: 8.0, command: "", selected: false, complexPattern: complex)
    }

    func testMissionLegsCarryArrowsOnTheSecondLegEverySixthAfterAndAtPatternBoundaries() throws {
        let plain = (0...14).map { leg($0) }
        let arrows = legArrows(plain, true)
        XCTAssertEqual(arrows.count, 2, "MissionController: the home-to-first leg has none, one when the count passes five, and the last leg always")
        let withSurvey = (0...4).map { leg($0, complex: $0 == 3) }
        XCTAssertEqual(legArrows(withSurvey, true).count, 2, "both legs touching a pattern get one")
        let first = try XCTUnwrap(arrows.first)
        XCTAssertTrue(first.at.latitude > 47.007 && first.at.latitude < 47.008, "arrows sit three quarters along the leg")
    }

    func testAWaypointsGimbalWedgePointsAtItsHeadingPlusTheGimbalYaw() {
        let looking = withChanges(leg(2)) {
            $0.heading = 90.0
            $0.gimbalYaw = 30.0
        }
        let bare = withChanges(leg(3)) { $0.heading = 90.0 }
        XCTAssertEqual(gimbalWedges([looking, bare]).map(\.bearing), [120.0])
    }

    func testAPatternItemGetsASecondMarkerAtItsExitLabelledWithItsLastSequenceNumber() {
        let survey = withChanges(leg(2, complex: true)) {
            $0.exit = TrackPoint(latitude: 47.5, longitude: 8.1)
            $0.foldedCommands = 6
        }
        let plain = withChanges(leg(3)) { $0.exit = TrackPoint(latitude: 47.6, longitude: 8.1) }
        let exits = exitMarkers([survey, plain])
        XCTAssertEqual(exits.map(\.0), [survey])
        XCTAssertEqual(exits.map(\.1), [TrackPoint(latitude: 47.5, longitude: 8.1)])
        let labels = missionFeatures([survey]).shapes.map { $0.getStringProperty(WAYPOINT_LABEL_PROPERTY) }
        XCTAssertEqual(labels, ["2", "8"], "TransectStyleMapVisuals labels the exit with lastSequenceNumber")
    }

    func testDraggingAFenceCornerLabelsEveryEdgeOfThatFenceWithItsLength() {
        let shape = EditableShape(
            path: "p",
            midpoints: [TrackPoint(latitude: 47.0, longitude: 8.0), TrackPoint(latitude: 47.1, longitude: 8.0)],
            splitInvokable: "",
            canRemoveVertex: true,
            edgeLengths: ["12.0 m", "8.5 m"]
        )
        let fence = FencePolygon(index: 0, inclusion: true, vertices: [], editable: shape)
        XCTAssertEqual(edgeLabels(.FenceVertex(polygon: 0, vertex: 1), [fence], []).map(\.text), ["12.0 m", "8.5 m"])
        XCTAssertEqual(edgeLabels(nil, [fence], []).count, 0, "nothing while no vertex is dragged")
    }

    func testAnRtlClosesTheRouteBackToHomeAndItsLegCarriesTheLastArrow() throws {
        let home = withChanges(leg(0)) { $0.closesRoute = true }
        let route = flownRoute([home, leg(1), leg(2)], false)
        XCTAssertEqual(route.map(\.index), [1, 2, 0])
        XCTAssertEqual(try XCTUnwrap(legArrows([home, leg(1), leg(2)], false).last).at.latitude, 47.0005, accuracy: 0.0002, "the last arrow sits on the leg home")
    }

    func testDraggingAPolygonsCentreMovesEveryCornerByTheSameOffset() throws {
        let square = [
            TrackPoint(latitude: 47.0, longitude: 8.0), TrackPoint(latitude: 47.0, longitude: 8.002),
            TrackPoint(latitude: 47.002, longitude: 8.002), TrackPoint(latitude: 47.002, longitude: 8.0),
        ]
        let centre = try XCTUnwrap(polygonCentre(square))
        XCTAssertEqual(centre.latitude, 47.001, accuracy: 1e-9)
        XCTAssertEqual(centre.longitude, 8.001, accuracy: 1e-9)
        let moved = try XCTUnwrap(shapeMovedTo(square, TrackPoint(latitude: 47.011, longitude: 8.001)))
        XCTAssertEqual(moved[0].latitude, 47.010, accuracy: 1e-6, "QGCMapPolygon::setCenter shifts each vertex by the centre's displacement")
        XCTAssertEqual(moved[0].longitude, 8.0, accuracy: 1e-6)
        XCTAssertNil(polygonCentre(Array(square.prefix(2))), "a line has no centre handle")
    }
}
