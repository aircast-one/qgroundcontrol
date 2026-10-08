import XCTest
@testable import Aircast

final class WaypointColourTests: XCTestCase {
    func testTheCommandsThatChangeWhatAPlanMeansEachGetTheirOwnColour() {
        XCTAssertEqual(waypointColour("takeoff", 22), TAKEOFF_COLOUR)
        XCTAssertEqual(waypointColour("land", 21), LAND_COLOUR)
        XCTAssertEqual(waypointColour("command", MAV_CMD_NAV_RETURN_TO_LAUNCH), RETURN_COLOUR)
        XCTAssertEqual(waypointColour("command", MAV_CMD_NAV_LOITER_TIME), LOITER_COLOUR)
    }

    func testTheLaunchPointIsNotDrawnAsSomewhereTheAircraftFliesTo() {
        XCTAssertEqual(waypointColour("settings", 16), START_COLOUR)
    }

    func testAnOrdinaryWaypointKeepsThePlainColour() {
        XCTAssertEqual(waypointColour("waypoint", 16), WAYPOINT_COLOUR)
        XCTAssertEqual(waypointColour("", 0), WAYPOINT_COLOUR)
    }

    func testAVtolTakeoffIsATakeoffBecauseTheKindSaysSoAndTheNameNeedNot() {
        XCTAssertEqual(waypointColour("takeoff", 84), TAKEOFF_COLOUR)
        XCTAssertEqual(waypointColour("land", 85), LAND_COLOUR)
    }

    func testAPlanInAnotherLanguageIsDrawnTheSameBecauseNoColourReadsAName() {
        let german = JSON.parse(
            #"{"items":[{"kind":"takeoff","command":22,"name":"Starten","flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}},{"kind":"command","command":20,"name":"Rückkehr zum Start","flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}}]}"#
        )
        XCTAssertEqual(missionItems(german).map { waypointColour($0.kind, $0.commandId) }, [TAKEOFF_COLOUR, RETURN_COLOUR])
    }
}
