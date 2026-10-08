import XCTest
@testable import Aircast

final class CircleRingTests: XCTestCase {
    private let centre = TrackPoint(41.0, 44.0)

    func testEveryPointSitsTheRequestedDistanceFromTheCentre() {
        let ring = circleRing(centre, 500.0, 12)
        XCTAssertEqual(12, ring.count)
        ring.forEach { XCTAssertEqual(500.0, metresBetween(centre, $0), accuracy: 1.0) }
    }

    func testTheRingSpreadsAllTheWayAround() throws {
        let ring = circleRing(centre, 500.0, 4)
        XCTAssertTrue(try XCTUnwrap(ring.map(\.latitude).max()) > centre.latitude)
        XCTAssertTrue(try XCTUnwrap(ring.map(\.latitude).min()) < centre.latitude)
        XCTAssertTrue(try XCTUnwrap(ring.map(\.longitude).max()) > centre.longitude)
        XCTAssertTrue(try XCTUnwrap(ring.map(\.longitude).min()) < centre.longitude)
    }

    func testACircleHighOnTheGlobeStillMeasuresCorrectly() {
        let arctic = TrackPoint(78.0, 15.0)
        circleRing(arctic, 2_000.0, 16).forEach { XCTAssertEqual(2_000.0, metresBetween(arctic, $0), accuracy: 2.0) }
    }

    func testACircleWithNoSizeOrTooFewSegmentsIsNotARing() {
        XCTAssertEqual(0, circleRing(centre, 0.0).count)
        XCTAssertEqual(0, circleRing(centre, -5.0).count)
        XCTAssertEqual(0, circleRing(centre, 100.0, 2).count)
    }

    func testCirclesBecomePolygonsThatKeepTheirIndexAndInclusion() {
        let polygons = circlesAsPolygons([
            FenceCircle(index: 2, inclusion: false, centre: centre, radius: 300.0),
            FenceCircle(index: 3, inclusion: true, centre: centre, radius: 0.0),
        ])
        XCTAssertEqual(1, polygons.count)
        XCTAssertEqual(2, polygons.first?.index)
        XCTAssertEqual(false, polygons.first?.inclusion)
        XCTAssertTrue((polygons.first?.vertices.count ?? 0) >= 3)
    }
}
