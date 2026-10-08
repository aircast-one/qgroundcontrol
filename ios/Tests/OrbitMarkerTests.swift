import XCTest
@testable import Aircast

final class OrbitMarkerTests: XCTestCase {
    func testTheCircleIsDrawnOnlyWhileTheVehicleOrbitsAsFlyViewMapShowsOrbitMapCircle() {
        let turning = orbitCircle(JSON.parse(#"{"class":"Orbit","orbiting":true,"centre":{"latitude":47.0,"longitude":8.0},"radiusMetres":50}"#))
        XCTAssertEqual(turning, OrbitCircle(centre: TrackPoint(latitude: 47.0, longitude: 8.0), radiusMetres: 50.0))
        XCTAssertEqual(orbitRing(turning).first, orbitRing(turning).last)
        XCTAssertNil(orbitCircle(JSON.parse(#"{"class":"Orbit","orbiting":false,"centre":null}"#)))
        XCTAssertNil(orbitCircle(nil))
        XCTAssertEqual(orbitRing(nil), [])
    }

    func testAnOrbitPreviewHasACentreHandleAndARadiusHandleDueEastThatDragsTheRadius() throws {
        let preview = OrbitCircle(centre: TrackPoint(latitude: 47.4, longitude: 8.5), radiusMetres: 30.0, clockwise: true)
        let handles = orbitHandles(preview)
        XCTAssertEqual(handles.first, preview.centre)
        let edge = try XCTUnwrap(handles.last)
        XCTAssertEqual(metresBetween(preview.centre, edge), 30.0, accuracy: 0.5)
        XCTAssertTrue(edge.longitude > preview.centre.longitude)
        XCTAssertEqual(draggedOrbitRadius(preview, edge), 30.0, accuracy: 0.5)
        XCTAssertEqual(draggedOrbitRadius(preview, preview.centre), MINIMUM_CIRCLE_RADIUS_METRES, accuracy: 0)
        XCTAssertTrue(orbitHandles(nil).isEmpty)
    }
}
