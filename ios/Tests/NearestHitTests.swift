import XCTest
@testable import Aircast

final class NearestHitTests: XCTestCase {
    func testTheClosestPointWinsRatherThanTheFirstDrawn() {
        XCTAssertEqual(1, nearestIndex(10, 10, [(100, 100), (12, 12), (60, 60)]))
    }

    func testATieGoesToTheOneDrawnFirst() {
        XCTAssertEqual(0, nearestIndex(10, 10, [(20, 10), (0, 10)]))
    }

    func testSomethingThatDidNotProjectSortsLast() {
        XCTAssertEqual(1, nearestIndex(10, 10, [nil, (40, 40)]))
    }

    func testButStillWinsWhenItIsAllThereIs() {
        XCTAssertEqual(0, nearestIndex(10, 10, [nil, nil]))
    }

    func testNothingUnderTheTapIsNoHit() {
        XCTAssertNil(nearestIndex(10, 10, []))
    }
}
