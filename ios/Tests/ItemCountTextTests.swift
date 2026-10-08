import XCTest
@testable import Aircast

final class ItemCountTextTests: XCTestCase {
    func testSingularForOne() {
        XCTAssertEqual("1 item", itemCountText(1))
    }

    func testPluralOtherwise() {
        XCTAssertEqual("0 items", itemCountText(0))
        XCTAssertEqual("6 items", itemCountText(6))
    }
}
