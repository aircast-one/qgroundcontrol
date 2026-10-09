import XCTest
@testable import Aircast

final class SettingStepperTests: XCTestCase {
    func testAStepMovesByItsSizeAndStopsAtTheEndsOfTheRange() {
        XCTAssertEqual([51.0, 49.0, 500.0, 0.0, -3.0], [
            steppedValue(50.0, 1.0, 1, 0.0...500.0),
            steppedValue(50.0, 1.0, -1, 0.0...500.0),
            steppedValue(500.0, 1.0, 1, 0.0...500.0),
            steppedValue(0.0, 1.0, -1, 0.0...500.0),
            steppedValue(-2.0, 1.0, -1, nil),
        ])
    }

    func testWholeNumbersReadWholeAndAHalfStepShowsItsDecimal() {
        XCTAssertEqual([0, 1, 1], [stepperDecimals(50.0, 1.0), stepperDecimals(5.0, 0.5), stepperDecimals(45.5, 1.0)])
        XCTAssertEqual(["50", "5.5", "\u{2014}"], [stepperText(50.0, 0), stepperText(5.5, 1), stepperText(nil, 0)])
    }

    func testTheAltitudeSliderSpansTheSameHeightInEitherUnit() {
        XCTAssertEqual(500.0, altitudeRange("m").upperBound, accuracy: 0)
        XCTAssertEqual(1640.0, altitudeRange("ft").upperBound, accuracy: 0)
    }

    func testTheWaypointStripLeavesOutThePlannedHomeWhichIsNotFlownTo() {
        let rows = [0, 1, 2].map { ItemRow(index: $0, number: "\($0)", name: "Waypoint", detail: "", colour: "#ffffff", placed: true) }
        XCTAssertEqual([1, 2], stripRows(rows).map(\.index))
    }
}
