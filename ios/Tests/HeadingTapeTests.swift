import XCTest
@testable import Aircast

final class HeadingTapeTests: XCTestCase {
    func testABearingIsPlacedRelativeToTheHeadingAcrossNorth() {
        XCTAssertEqual(relativeBearing(10, 350), 20, accuracy: 0.001)
        XCTAssertEqual(relativeBearing(350, 10), -20, accuracy: 0.001)
        XCTAssertEqual(relativeBearing(180, 0), -180, accuracy: 0.001)
        XCTAssertEqual(relativeBearing(725, 5), 0, accuracy: 0.001)
    }

    func testTheHeadingSitsInTheMiddleAndTheSpanFillsTheWidth() {
        XCTAssertEqual(tapeX(0, 320), 160, accuracy: 0.001)
        XCTAssertEqual(tapeX(-60, 320), 0, accuracy: 0.001)
        XCTAssertEqual(tapeX(60, 320), 320, accuracy: 0.001)
    }

    func testCardinalPointsAreLettersAndTheRestAreDegrees() {
        XCTAssertEqual(tapeLabel(360), "N")
        XCTAssertEqual(tapeLabel(90), "E")
        XCTAssertEqual(tapeLabel(-90), "W")
        XCTAssertEqual(tapeLabel(-30), "330")
        XCTAssertEqual(tapeLabel(120), "120")
    }

    func testTicksCoverTheVisibleSpanInTenDegreeSteps() {
        let ticks = tapeTicks(102)
        XCTAssertEqual(ticks.first, 50)
        XCTAssertEqual(ticks.last, 160)
        XCTAssertEqual(ticks.count, 12)
    }

    func testLabelsClearTheReadoutAndTheEdges() {
        XCTAssertFalse(tapeLabelShown(12))
        XCTAssertTrue(tapeLabelShown(-18))
        XCTAssertTrue(tapeLabelShown(48))
        XCTAssertFalse(tapeLabelShown(58))
    }

    func testTheReadoutIsWholeDegrees() {
        XCTAssertEqual(headingReadout(7.4), "7°")
        XCTAssertEqual(headingReadout(359.6), "0°")
        XCTAssertEqual(headingReadout(-90), "270°")
    }
}
