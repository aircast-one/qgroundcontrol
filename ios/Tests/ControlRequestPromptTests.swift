import XCTest
@testable import Aircast

final class ControlRequestPromptTests: XCTestCase {
    func testAnotherStationsRequestAndTheTakeoverCountdownAreReadFromTheCore() {
        let asked = controlPrompt(JSON.parse(#"{"inControl":true,"incomingRequest":{"systemId":9,"timeoutMs":20000,"remainingMs":12500},"takeoverRevertMs":null}"#))
        XCTAssertEqual(asked.incoming, IncomingControlRequest(systemId: 9, timeoutMs: 20000, remainingMs: 12500))
        XCTAssertNil(asked.revertMs)
        XCTAssertEqual(secondsLeft(12500), 13)
        XCTAssertEqual(controlPrompt(JSON.parse(#"{"inControl":true,"incomingRequest":null,"takeoverRevertMs":4000}"#)).revertMs, 4000)
        XCTAssertNil(controlPrompt(JSON.parse(#"{"inControl":false,"incomingRequest":null,"takeoverRevertMs":4000}"#)).revertMs, "the countdown closes once the other station has taken control")
    }
}
