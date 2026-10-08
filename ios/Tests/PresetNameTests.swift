import XCTest
@testable import Aircast

final class PresetNameTests: XCTestCase {
    func testTheNameIsCheckedAsItIsTypedLikeTransectStyleComplexItemEditor() {
        XCTAssertEqual("Preset name cannot be blank.", presetNameError("  "))
        XCTAssertEqual("Preset name cannot include the \"/\" character.", presetNameError("a/b"))
        XCTAssertNil(presetNameError("Farm 2"))
    }
}
