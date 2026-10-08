import XCTest
@testable import Aircast

final class TuningChartTests: XCTestCase {
    func testHistoryKeepsThreeMinutes() {
        let series = (0...200).reduce([Sample]()) { list, s in withSample(list, Sample(seconds: Double(s), value: Double(s))) }
        XCTAssertEqual(series.first?.seconds, 20.0)
    }

    func testTheYRangeOnlyGrowsSnappedOutwardToFivesAsPidTuningsAdjustYAxisDoes() {
        XCTAssertNil(grownRange(nil, []))
        let first = grownRange(nil, [3.0])
        XCTAssertEqual(first?.0, 3.0, "the first point sets both ends")
        XCTAssertEqual(first?.1, 3.0, "the first point sets both ends")
        let grown = grownRange((3.0, 3.0), [-1.0, 7.0])
        XCTAssertEqual(grown?.0, -10.0)
        XCTAssertEqual(grown?.1, 10.0)
        let kept = grownRange((-10.0, 10.0), [2.0])
        XCTAssertEqual(kept?.0, -10.0, "a value inside the range never shrinks it")
        XCTAssertEqual(kept?.1, 10.0, "a value inside the range never shrinks it")
    }
}
