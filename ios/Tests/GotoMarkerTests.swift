import XCTest
@testable import Aircast

final class GotoMarkerTests: XCTestCase {
    func testTheGoHereMarkerCarriesALoiterRingOnlyWhenTheCoreSendsARadius() throws {
        let fixedWing = gotoLocation(JSON.parse(#"{"gotoLocation":{"latitude":47.4,"longitude":8.5,"loiterRadiusMetres":80.0}}"#))
        XCTAssertEqual(fixedWing, GotoLocation(at: TrackPoint(latitude: 47.4, longitude: 8.5), loiterRadiusMetres: 80.0))
        let ring = gotoRing(fixedWing)
        XCTAssertEqual(ring.first, ring.last)
        XCTAssertTrue(ring.count > 3)
        let copter = gotoLocation(JSON.parse(#"{"gotoLocation":{"latitude":47.4,"longitude":8.5,"loiterRadiusMetres":null}}"#))
        XCTAssertEqual(copter, GotoLocation(at: TrackPoint(latitude: 47.4, longitude: 8.5), loiterRadiusMetres: nil))
        XCTAssertTrue(gotoRing(copter).isEmpty)
        XCTAssertNil(gotoLocation(JSON.parse(#"{"gotoLocation":null}"#)))
    }

    func testTheLoiterRingPointsItsRotationArrowsTheWayTheCircleWasCommitted() throws {
        let anticlockwise = try XCTUnwrap(gotoLocation(JSON.parse(#"{"gotoLocation":{"latitude":47.4,"longitude":8.5,"loiterRadiusMetres":80.0,"loiterClockwise":false}}"#)))
        XCTAssertEqual(gotoArrows(anticlockwise).map(\.1), [270.0, 90.0])
        XCTAssertEqual(gotoArrows(withChanges(anticlockwise) { $0.loiterClockwise = true }).map(\.1), [90.0, 270.0])
        XCTAssertTrue(gotoArrows(withChanges(anticlockwise) { $0.loiterRadiusMetres = nil }).isEmpty)
    }

    func testARadiusEditRedrawsTheLoiterRingWithADragHandleDueEastNeverBelowTheCircleMinimum() throws {
        let committed = GotoLocation(at: TrackPoint(latitude: 47.4, longitude: 8.5), loiterRadiusMetres: 80.0, loiterRadiusText: "80 m", loiterClockwise: true)
        let edit = LoiterEdit(radiusMetres: 152.4, clockwise: false, unit: "ft", metresPerUnit: 0.3048)
        let shown = editedGoto(committed, edit)
        XCTAssertEqual(shown, GotoLocation(at: TrackPoint(latitude: 47.4, longitude: 8.5), loiterRadiusMetres: 152.4, loiterRadiusText: "500 ft", loiterClockwise: false))
        XCTAssertEqual(editedGoto(committed, nil), committed)
        let unringed = withChanges(committed) { $0.loiterRadiusMetres = nil }
        XCTAssertEqual(editedGoto(unringed, edit), unringed)
        let handle = try XCTUnwrap(gotoRadiusHandle(shown))
        XCTAssertEqual(metresBetween(committed.at, handle), 152.4, accuracy: 0.5)
        XCTAssertTrue(handle.longitude > committed.at.longitude)
        XCTAssertEqual(handle.latitude, 47.4, accuracy: 1e-4)
        XCTAssertEqual(draggedGotoRadius(committed, handle), 152.4, accuracy: 0.5)
        XCTAssertEqual(draggedGotoRadius(committed, committed.at), MINIMUM_CIRCLE_RADIUS_METRES, accuracy: 0)
        XCTAssertNil(gotoRadiusHandle(unringed))
    }
}
