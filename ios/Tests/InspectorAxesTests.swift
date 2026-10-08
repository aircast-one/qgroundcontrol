import XCTest
@testable import Aircast

final class InspectorAxesTests: XCTestCase {
    func testTheTimeAxisTicksEveryThirdOfTheWindowInLocalMmSs() throws {
        let utc = try XCTUnwrap(TimeZone(identifier: "UTC"))
        let ticks = timeTicks(30_000, 125_000, zone: utc)
        XCTAssertEqual(ticks.map(\.0), [0, Float(1) / 3, Float(2) / 3, 1])
        XCTAssertEqual(ticks.map(\.1), ["01:35", "01:45", "01:55", "02:05"])
        XCTAssertEqual(timeTicks(30_000, 125_000, zone: try XCTUnwrap(TimeZone(identifier: "Asia/Kolkata"))).last?.1, "32:05")
    }

    func testValueTicksClimbFromTheBottomOfTheRangeToTheTop() throws {
        let ticks = valueTicks(0.0, 100.0)
        let first = try XCTUnwrap(ticks.first)
        let last = try XCTUnwrap(ticks.last)
        XCTAssertEqual(first.1, "0.000")
        XCTAssertEqual(first.0, 1)
        XCTAssertEqual(last.1, "100.0")
        XCTAssertEqual(last.0, 0)
    }
}
