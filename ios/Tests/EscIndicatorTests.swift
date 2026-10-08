import XCTest
@testable import Aircast

final class EscIndicatorTests: XCTestCase {
    func testTheCellAndPageReadTheCoresEscSummary() throws {
        let summary = try XCTUnwrap(escSummary(JSON.parse(
            #"{"shown":true,"onlineCount":4,"healthy":true,"healthText":"OK","healthyMotorsText":"4/4","totalErrors":3,"#
                + #""motors":[{"title":"Motor 1","healthy":true,"rpm":"4200 rpm","temperature":"","voltage":"16 V","current":"","errors":"1"}]}"#
        )))
        XCTAssertEqual(escCellText(summary), "ESC 4 OK")
        XCTAssertEqual(summary.motors[0].title, "Motor 1")
        XCTAssertEqual(summary.motors[0].rows[0].0, "RPM")
        XCTAssertEqual(summary.motors[0].rows[0].1, "4200 rpm")
        XCTAssertNil(escSummary(JSON.parse(#"{"shown":false}"#)))
    }
}
