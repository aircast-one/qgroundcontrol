import XCTest
@testable import Aircast

final class ControlKindTests: XCTestCase {
    func testTheHeadKnowsExactlyTheKindsTheCoreEnumerates() {
        XCTAssertEqual(KNOWN_CONTROL_KINDS, ["toggle", "choice", "bitmask", "text", "number", "label", "dialog", "button"])
    }

    func testEveryEnumeratedKindIsRenderedRatherThanRefused() {
        XCTAssertTrue(KNOWN_CONTROL_KINDS.allSatisfy(controlIsUnderstood))
    }

    func testAKindTheCoreAddsLaterIsNotGivenAnEditableControl() {
        XCTAssertFalse(controlIsUnderstood("slider"))
        XCTAssertFalse(controlIsUnderstood("colour"))
    }

    func testAControlWithNoKindAtAllIsStillEditableBecauseThatIsEveryOtherPath() {
        XCTAssertTrue(controlIsUnderstood(""))
    }

    func testTheKindIsCarriedFromTheProjectionOntoTheFact() {
        let control = JSON.parse(#"{"label":"Arm Checks","name":"ARMING_CHECK","path":"p","control":"bitmask","value":82,"valueString":"82"}"#)
        XCTAssertEqual(factFromControl(control)?.controlKind, "bitmask")
    }
}
