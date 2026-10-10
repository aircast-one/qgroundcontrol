import XCTest
@testable import Aircast

final class DeepLinkSetupPromptTests: XCTestCase {
    func testADeviceLinkIsShownOnlyWhileTheCoreHoldsOne() {
        XCTAssertNil(deepLinkSetup(JSON.parse(#"{"show":false,"host":null}"#)))
        XCTAssertNil(deepLinkSetup(nil))
        let shown = deepLinkSetup(JSON.parse(#"{"show":true,"title":"Set up from this device?","text":"from 10.0.0.5","accept":"Set up","decline":"Ignore"}"#))
        XCTAssertEqual(shown, DeepLinkSetup(title: "Set up from this device?", text: "from 10.0.0.5", accept: "Set up", decline: "Ignore"))
    }
}
