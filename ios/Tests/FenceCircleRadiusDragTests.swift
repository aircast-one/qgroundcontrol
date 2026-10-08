import XCTest
@testable import Aircast

final class FenceCircleRadiusDragTests: XCTestCase {
    private let centre = TrackPoint(latitude: 47.0, longitude: 8.0)

    func testTheEdgeHandleSitsEastOfTheCentreAtTheRadiusAsQgcMapCircleVisualsPlacesItsDragHandle() throws {
        let circle = FenceCircle(index: 3, inclusion: true, centre: centre, radius: 100.0)
        let edge = circleEdge(circle)
        XCTAssertEqual(metresBetween(centre, edge), 100.0, accuracy: 0.5)
        XCTAssertTrue(edge.longitude > centre.longitude)
        let handle = try XCTUnwrap(vertexHandleFeatures([], [], circles: [circle]).shapes.first { $0.getStringProperty(HANDLE_KIND_PROPERTY) == HANDLE_KIND_FENCE_CIRCLE_RADIUS })
        XCTAssertEqual(
            handleHit(
                handle.getStringProperty(HANDLE_KIND_PROPERTY),
                Int(try XCTUnwrap(handle.getNumberProperty(POLYGON_INDEX_PROPERTY))),
                Int(try XCTUnwrap(handle.getNumberProperty(VERTEX_INDEX_PROPERTY)))
            ),
            .CircleRadius(index: 3)
        )
    }

    func testDraggingSetsTheDistanceFromTheCentreInTheUnitsTheRadiusIsShownIn() {
        let feet = FenceCircle(index: 0, inclusion: true, centre: centre, radius: 328.084, radiusMetres: 100.0, radiusUnits: "ft")
        XCTAssertEqual(draggedCircleRadius(feet, pointAt(centre, 200.0, 90.0)), 656.168, accuracy: 0.5)
    }

    func testADragPastTheFenceLimitsStopsAtThem() {
        let bounded = FenceCircle(index: 0, inclusion: true, centre: centre, radius: 100.0, radiusMinimum: 10.0, radiusMaximum: 500.0)
        XCTAssertEqual(draggedCircleRadius(bounded, pointAt(centre, 1.0, 90.0)), 10.0, accuracy: 1e-9)
        XCTAssertEqual(draggedCircleRadius(bounded, pointAt(centre, 900.0, 90.0)), 500.0, accuracy: 1e-9)
        let unbounded = FenceCircle(index: 0, inclusion: true, centre: centre, radius: 100.0)
        XCTAssertEqual(draggedCircleRadius(unbounded, centre), 0.1, accuracy: 1e-9, "a drag onto the centre keeps QGCMapCircle's 0.1 m so the circle is not dropped")
    }
}
