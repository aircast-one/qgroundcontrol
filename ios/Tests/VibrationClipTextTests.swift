import XCTest
@testable import Aircast

final class VibrationClipTextTests: XCTestCase {
    func testEachClipCountNamesItsAccelerometerLikeVibrationPage() {
        XCTAssertEqual(clipText([0, 3, 0]), "Accel 1: 0 · Accel 2: 3 · Accel 3: 0")
    }
}
