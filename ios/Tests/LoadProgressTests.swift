import XCTest
@testable import Aircast

final class LoadProgressTests: XCTestCase {
    func testTheChipShowsLoadProgressUntilTheInitialConnectCompletes() {
        XCTAssertEqual(loadingProgress(.bool(false), .number(0.4)), 0.4)
        XCTAssertEqual(loadingProgress(.bool(false), nil), 0)
        XCTAssertEqual(loadingProgress(.bool(false), .number(3)), 1)
        XCTAssertNil(loadingProgress(.bool(true), .number(1.0)))
        XCTAssertNil(loadingProgress(nil, nil))
    }
}
