import XCTest
@testable import Aircast

private let ITEMS_ADDED = #"""
{"class": "AltitudeModes", "context": "mission", "current": 1, "modes": [
  {"raw": 1, "title": "Relative To Launch", "enabled": true, "current": true},
  {"raw": 2, "title": "AMSL", "enabled": false, "current": false, "reason": "locked"},
  {"raw": 0, "title": "Mixed Modes", "enabled": true, "current": false}
 ], "omitted": []}
"""#

private let MIXED_ONLY = #"""
{"class": "AltitudeModes", "context": "mission", "current": 0, "modes": [
  {"raw": 1, "title": "Relative To Launch", "enabled": false, "current": false},
  {"raw": 0, "title": "Mixed Modes", "enabled": true, "current": true}
 ], "omitted": []}
"""#

final class MissionAltitudeFrameTests: XCTestCase {
    func testTheMissionFrameIsReadFromThePlanView() {
        XCTAssertEqual(1, globalAltitudeFrame(JSON.parse(#"{"globalAltitudeFrame":1}"#)))
        XCTAssertNil(globalAltitudeFrame(JSON.parse(#"{"globalAltitudeFrame":null}"#)))
    }

    func testMixedStaysPickableForThePlanOnceItemsExistLikeMissionSettingsEditorsAltModeMenu() {
        let view = altitudeModesView(JSON.parse(ITEMS_ADDED))
        XCTAssertEqual([1, 2, 0], missionFramePicks(view).map(\.raw))
        XCTAssertTrue(missionFrameChoice(view))
    }

    func testOnlyTheCurrentModeEnabledLeavesNothingToPick() {
        XCTAssertFalse(missionFrameChoice(altitudeModesView(JSON.parse(MIXED_ONLY))))
    }
}
