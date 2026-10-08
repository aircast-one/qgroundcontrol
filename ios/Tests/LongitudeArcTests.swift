import XCTest
@testable import Aircast

final class LongitudeArcTests: XCTestCase {
    private func arc(_ longitudes: [Double]) -> [Double] {
        let (west, east) = longitudeArc(longitudes)
        return [west, east]
    }

    func testAPlanEitherSideOfTheAntimeridianKeepsTheShortArcAcrossIt() {
        XCTAssertEqual([170.0, -170.0], arc([170.0, -170.0]))
    }

    func testAnOrdinaryPlanSpansWestToEastTheObviousWay() {
        XCTAssertEqual([10.0, 20.0], arc([20.0, 10.0]))
        XCTAssertEqual([-20.0, -10.0], arc([-10.0, -20.0]))
    }

    func testOnePointAndNoPointsAreBothAnswerable() {
        XCTAssertEqual([42.0, 42.0], arc([42.0]))
        XCTAssertEqual([42.0, 42.0], arc([42.0, 42.0]))
        XCTAssertEqual([0.0, 0.0], arc([]))
    }

    func testALongitudePastTheWrapIsBroughtBackBeforeAnythingIsCompared() {
        XCTAssertEqual(180.0, normaliseLongitude(-180.0), accuracy: 0)
        XCTAssertEqual(-179.0, normaliseLongitude(181.0), accuracy: 0)
        XCTAssertEqual(179.0, normaliseLongitude(-181.0), accuracy: 0)
        XCTAssertEqual(0.0, normaliseLongitude(360.0), accuracy: 0)
    }

    func testThreePointsStraddlingTheWrapStillTakeTheArcContainingThem() {
        XCTAssertEqual([10.0, -175.0], arc([175.0, -175.0, 10.0]))
    }
}
