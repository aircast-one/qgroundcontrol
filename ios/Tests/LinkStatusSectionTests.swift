import XCTest
@testable import Aircast

final class LinkStatusSectionTests: XCTestCase {
    func testLinkStatusRowsReadInOrder() {
        let view = JSON.parse(#"{"rows":[{"label":"Messages Sent","value":"62"},{"label":"Loss Rate","value":"3%"}]}"#)
        XCTAssertEqual(linkStatusRows(view).map { [$0.0, $0.1] }, [["Messages Sent", "62"], ["Loss Rate", "3%"]])
        XCTAssertTrue(linkStatusRows(nil).isEmpty)
    }
}
