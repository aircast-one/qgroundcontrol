import XCTest
@testable import Aircast

final class ItemCameraDuplicateTests: XCTestCase {
    private func camera(_ commandsGimbal: Bool) -> JSON {
        JSON.parse(#"{"available":true,"commandsGimbal":\#(commandsGimbal),"cameraAction":{"choices":["No change","Take photo"],"choice":0,"text":"No change"},"gimbalPitch":{"text":"-90 deg"},"gimbalYaw":{"text":"45 deg"}}"#)
    }

    func testTheSummaryIsDroppedWhenThePickerBesideItAlreadySaysTheSameWord() {
        let view = camera(false)
        XCTAssertEqual("No change", itemCameraText(view))
        XCTAssertNil(itemCameraTextBeside(view, "No change"))
    }

    func testTheSummaryStaysWhenItCarriesTheGimbalThePickerCannotShow() {
        XCTAssertEqual("No change · gimbal -90 deg / 45 deg", itemCameraTextBeside(camera(true), "No change"))
    }

    func testWithNoPickerAtAllTheSummaryIsWhateverItWas() {
        XCTAssertEqual("No change", itemCameraTextBeside(camera(false), nil))
    }
}
