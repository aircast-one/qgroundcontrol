import XCTest
@testable import Aircast

final class MarkerDensityTests: XCTestCase {
    func testAnOrdinaryPlanIsNotCrowdedAndIsDrawnExactlyAsBefore() {
        XCTAssertFalse(crowded(4))
        XCTAssertFalse(crowded(CROWDED_ITEMS))
        XCTAssertEqual(markerRadius(false, false), MARKER_RADIUS, accuracy: 0)
        XCTAssertEqual(waypointLabel(7, false), "7")
    }

    func testAMissionDownloadedOffAVehicleIsCrowdedAndDropsToDots() {
        XCTAssertTrue(crowded(CROWDED_ITEMS + 1))
        XCTAssertTrue(crowded(212))
        XCTAssertEqual(markerRadius(true, false), CROWDED_RADIUS, accuracy: 0)
    }

    func testNumberingIsDroppedOnlyWhenThereAreTooManyToRead() {
        XCTAssertEqual(waypointLabel(150, true), "")
        XCTAssertEqual(waypointLabel(150, false), "150")
    }

    func testTheSelectedItemStaysFullSizeSoItCanStillBeFoundInACrowd() {
        XCTAssertEqual(markerRadius(true, true), MARKER_RADIUS, accuracy: 0)
    }

    func testASelectedItemKeepsItsHeavyOutlineWhateverTheCrowd() {
        XCTAssertEqual(markerStroke(false, true), markerStroke(true, true), accuracy: 0)
        XCTAssertTrue(markerStroke(true, false) < markerStroke(false, false))
    }

    func testEveryWaypointCarriesItsOwnSizeSoTheLayerNeedsNoZoomRule() throws {
        let items = (0...80).map {
            MissionItem(index: $0, sequence: $0, latitude: 41.0 + Double($0) * 0.001, longitude: 44.0, command: "Waypoint", selected: false, kind: "waypoint")
        }
        let features = missionFeatures(items, selectedIndex: 3).shapes
        XCTAssertEqual(features.count, 81)
        XCTAssertEqual(try XCTUnwrap(features[0].getNumberProperty(WAYPOINT_RADIUS_PROPERTY)), CROWDED_RADIUS, accuracy: 0)
        XCTAssertEqual(try XCTUnwrap(features[3].getNumberProperty(WAYPOINT_RADIUS_PROPERTY)), MARKER_RADIUS, accuracy: 0)
        XCTAssertEqual(features[0].getStringProperty(WAYPOINT_LABEL_PROPERTY), "")
    }
}
