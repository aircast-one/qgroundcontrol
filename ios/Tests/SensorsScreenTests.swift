import XCTest
@testable import Aircast

final class SensorsScreenTests: XCTestCase {
    func testTheProgressRingTruncatesToAWholePercentAndNeverTrapsOnAWildValue() {
        XCTAssertEqual(progressPercent(0.256), 25)
        XCTAssertEqual(progressPercent(1), 100)
        XCTAssertEqual(progressPercent(.nan), 0)
        XCTAssertEqual(progressPercent(1e300), 0)
    }
}
