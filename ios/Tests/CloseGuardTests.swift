import XCTest
@testable import Aircast

final class CloseGuardTests: XCTestCase {
    func testClosePromptsAreReadInTheOrderTheCoreAsksThem() {
        XCTAssertEqual(closePrompts(JSON.parse(#"{"prompts":[{"id":"unsavedMission","message":"A"},{"id":"activeConnections","message":"B"}]}"#)), ["A", "B"])
        XCTAssertEqual(closePrompts(nil), [])
    }
}
