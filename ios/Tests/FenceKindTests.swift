import XCTest
@testable import Aircast

final class FenceKindTests: XCTestCase {
    private let view = JSON.parse(#"{"kind":"object","polygons":[{"index":0,"inclusion":false,"kindText":"Polygon 1","detailText":"3 vertices · 2.5 ha","vertices":[{"latitude":41.0,"longitude":44.0},{"latitude":41.1,"longitude":44.0},{"latitude":41.1,"longitude":44.1}]}],"circles":[{"index":0,"inclusion":true,"kindText":"Circle 1","centre":{"latitude":41.0,"longitude":44.0},"radius":150.0}]}"#)
    private var polygons: [FencePolygon] { fencePolygons(view) }
    private var circles: [FenceCircle] { fenceCircles(view) }

    func testASelectedFenceSaysWhetherItKeepsTheVehicleInOrOut() {
        XCTAssertEqual("Circle 1 \u{00b7} Inclusion", fenceDetail(.Circle(index: 0), polygons, circles))
        XCTAssertEqual("Circle 1 \u{00b7} Inclusion", fenceDetail(.CircleCentre(index: 0), polygons, circles))
    }

    func testAFenceSaysWhatItIsAndHowBigNotOneOrTheOther() {
        XCTAssertEqual("Polygon 1 \u{00b7} Exclusion \u{00b7} 3 vertices \u{00b7} 2.5 ha", fenceDetail(.FenceVertex(polygon: 0, vertex: 0), polygons, circles))
    }

    func testACircleDrawnAsAPolygonKeepsTheWordingItWasGiven() {
        XCTAssertEqual(["Circle 1"], circlesAsPolygons(circles).map(\.kindText))
    }

    func testASelectionThatIsNotAFenceSaysNothing() {
        XCTAssertNil(fenceDetail(.Waypoint(index: 0), polygons, circles))
        XCTAssertNil(fenceDetail(nil, polygons, circles))
        XCTAssertNil(fenceDetail(.FenceVertex(polygon: 9, vertex: 0), polygons, circles))
    }
}
