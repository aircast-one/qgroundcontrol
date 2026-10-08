import XCTest
@testable import Aircast

final class MavlinkActionsTests: XCTestCase {
    func testTheFilesAndFlyViewActionsReadFromTheCore() {
        XCTAssertNil(mavlinkActions(JSON.parse("{}")))
        let read = mavlinkActions(JSON.parse(#"{"files":["fly.json"],"flyViewFile":"fly.json","joystickFile":"","flyViewPath":"a","joystickPath":"b","actions":[{"label":"Lights","description":"toggle"}]}"#))!
        XCTAssertEqual(read.actions, [MavlinkActionEntry(label: "Lights", description: "toggle")])
        XCTAssertEqual(chosenFile(NO_ACTIONS_FILE), "")
        XCTAssertEqual(chosenFile("fly.json"), "fly.json")
    }
}
