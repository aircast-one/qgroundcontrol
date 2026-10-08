import XCTest
@testable import Aircast

final class InsertAfterSelectionTests: XCTestCase {
    func testAnySelectedMissionItemIsCurrentAsPlanViewInsertsAfterCurrentPlanViewVIIndex() {
        XCTAssertEqual(4, missionItemIndex(.SurveyVertex(item: 4, vertex: 2)))
        XCTAssertEqual(5, missionItemIndex(.ShapeCentre(fence: false, owner: 5)))
        XCTAssertNil(missionItemIndex(.ShapeCentre(fence: true, owner: 5)), "a fence shape is not a mission item")
        XCTAssertEqual(6, missionItemIndex(.LandingPlace(index: 6, place: 0)))
        XCTAssertNil(missionItemIndex(.Rally(index: 1)))
    }

    func testAnItemNotReadyToSendShowsAQuestionMarkSealLikeMissionItemEditor() {
        let item = MissionItem(index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0, kind: "waypoint", commandId: 16)
        XCTAssertEqual(["2"], itemRows([item]).map(\.number))
        var unready = item
        unready.readyForSave = false
        XCTAssertEqual([NOT_READY_SEAL], itemRows([unready]).map(\.number))
    }
}
