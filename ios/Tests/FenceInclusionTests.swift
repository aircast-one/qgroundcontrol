import XCTest
@testable import Aircast

final class FenceInclusionTests: XCTestCase {
    private func polygon(_ index: Int, _ inclusion: Bool) -> FencePolygon {
        FencePolygon(index: index, inclusion: inclusion, vertices: (0..<4).map { TrackPoint(41.0 + Double($0), 44.0) }, kindText: "Polygon \(index + 1)")
    }

    private func circle(_ index: Int, _ inclusion: Bool) -> FenceCircle {
        FenceCircle(index: index, inclusion: inclusion, centre: TrackPoint(41.0, 44.0), radius: 150.0, kindText: "Circle \(index + 1)")
    }

    func testAKeepInFenceOffersToBecomeKeepOutAndTheOtherWay() {
        XCTAssertEqual(true, selectedFence(.FenceVertex(polygon: 0, vertex: 0), [polygon(0, true)], [])?.keepsIn)
        XCTAssertEqual(false, selectedFence(.FenceVertex(polygon: 0, vertex: 0), [polygon(0, false)], [])?.keepsIn)
    }

    func testACircleIsNamedByItsEdgeOrItsCentreAndOnlyDescribedLikeGeoFenceEditor() {
        let circles = [circle(2, false)]
        XCTAssertEqual(false, selectedFence(.Circle(index: 2), [], circles)?.keepsIn)
        XCTAssertEqual(false, selectedFence(.CircleCentre(index: 2), [], circles)?.keepsIn)
        XCTAssertNil(selectedFence(.Circle(index: 2), [], circles)?.flip)
        XCTAssertEqual(true, selectedFence(.FenceVertex(polygon: 0, vertex: 0), [polygon(0, true)], [])?.flip != nil)
    }

    func testASelectionThatIsNotAFenceOffersNoFlip() {
        XCTAssertNil(selectedFence(.Waypoint(index: 0), [polygon(0, true)], []))
        XCTAssertNil(selectedFence(nil, [polygon(0, true)], []))
        XCTAssertNil(selectedFence(.FenceVertex(polygon: 9, vertex: 0), [polygon(0, true)], []))
    }

    func testANewFenceIsSizedFromTheVisibleMapLikeGeoFenceEditorsViewportCorners() {
        let view = [TrackPoint(48.0, 8.0), TrackPoint(48.0, 9.0), TrackPoint(47.0, 9.0), TrackPoint(47.0, 8.0)]
        let window = fenceWindow(view, TrackPoint(47.5, 8.5))
        XCTAssertEqual(TrackPoint(48.0, 8.0), window.0)
        XCTAssertEqual(TrackPoint(47.0, 9.0), window.1)
        let fallback = fenceWindow([], TrackPoint(47.5, 8.5))
        XCTAssertEqual(47.502, fallback.0.latitude, accuracy: 1e-12)
        XCTAssertEqual(8.498, fallback.0.longitude, accuracy: 1e-12)
        XCTAssertEqual(47.498, fallback.1.latitude, accuracy: 1e-12)
        XCTAssertEqual(8.502, fallback.1.longitude, accuracy: 1e-12)
    }
}
