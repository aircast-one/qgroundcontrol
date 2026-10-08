import XCTest
@testable import Aircast

final class ItemCameraNoteTests: XCTestCase {
    func testTheMissionStartNoteIsShownWhenTheCoreServesItLikeMissionSettingsEditor() {
        let note = "Camera commands above take effect immediately at mission start."
        XCTAssertEqual(note, itemCameraNote(JSON.parse(#"{"available":true,"note":"\#(note)"}"#)))
        XCTAssertNil(itemCameraNote(JSON.parse(#"{"available":true,"note":null}"#)))
        XCTAssertNil(itemCameraNote(JSON.parse(#"{"available":false,"note":"\#(note)"}"#)))
    }
}
