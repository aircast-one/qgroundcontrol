import XCTest
@testable import Aircast

final class ItemEditorTitleTests: XCTestCase {
    func testTheEditorHeadsWithTheSequenceAndCommandNameLikeMissionItemEditor() {
        XCTAssertEqual("#3 Waypoint", itemEditorTitle(JSON.parse(#"{"commandName":"Waypoint","sequenceNumber":3}"#), 2))
        XCTAssertEqual("Item 2", itemEditorTitle(nil, 2))
    }
}
