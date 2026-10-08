import XCTest
@testable import Aircast

final class EscCalibrationTests: XCTestCase {
    func testTheDialogShowsTheCoresHighlightedPrefixAndText() throws {
        let state = try XCTUnwrap(escCalibration(JSON.parse(#"{"open":true,"highlight":"ESC Calibration failed. ","text":"timeout","running":false}"#)))
        XCTAssertEqual(state, EscCalibrationState(highlight: "ESC Calibration failed. ", text: "timeout", running: false))
        XCTAssertNil(escCalibration(JSON.parse(#"{"open":false}"#)))
    }
}
