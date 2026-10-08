import XCTest
@testable import Aircast

final class WaypointSelectionTests: XCTestCase {
    private func items() -> [MissionItem] {
        missionItems(JSON.parse(
            #"{"kind":"object","items":[{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}},{"flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}}]}"#
        ))
    }

    func testOnlyTheSelectedWaypointIsMarked() {
        let features = missionFeatures(items(), selectedIndex: 1).shapes
        XCTAssertFalse(features[0].getBooleanProperty(WAYPOINT_SELECTED_PROPERTY))
        XCTAssertTrue(features[1].getBooleanProperty(WAYPOINT_SELECTED_PROPERTY))
    }

    func testNothingIsMarkedWhenNothingIsSelected() {
        let features = missionFeatures(items(), selectedIndex: nil).shapes
        XCTAssertTrue(features.allSatisfy { !$0.getBooleanProperty(WAYPOINT_SELECTED_PROPERTY) })
    }

    func testEveryFeatureCarriesTheFlagSoTheLayerNeverReadsAMissingProperty() {
        let features = missionFeatures(items(), selectedIndex: 99).shapes
        XCTAssertTrue(features.allSatisfy { $0.hasProperty(WAYPOINT_SELECTED_PROPERTY) })
    }
}
