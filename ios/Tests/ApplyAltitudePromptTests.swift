import XCTest
@testable import Aircast

final class ApplyAltitudePromptTests: XCTestCase {
    func testThePromptShowsOnlyWhileTheCoreAsks() {
        XCTAssertEqual(AltitudePrompt(title: "Apply new altitude", text: "t"), applyAltitudePrompt(JSON.parse(#"{"applyAltitudePrompt":{"title":"Apply new altitude","text":"t"}}"#)))
        XCTAssertNil(applyAltitudePrompt(JSON.parse(#"{"applyAltitudePrompt":null}"#)))
    }
}
