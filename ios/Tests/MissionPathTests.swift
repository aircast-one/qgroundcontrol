import MapLibre
import XCTest
@testable import Aircast

final class MissionPathTests: XCTestCase {
    private func item(_ index: Int, _ longitude: Double) -> MissionItem {
        MissionItem(index: index, sequence: index + 1, latitude: 41.0, longitude: longitude, command: "Waypoint", selected: false, altitude: .nan)
    }

    private lazy var home = item(0, 44.0)
    private lazy var first = item(1, 44.1)
    private lazy var second = item(2, 44.2)

    private func longitudes(_ items: [MissionItem], _ link: Bool) -> [Double]? {
        (missionPath(items, link) as? MLNMultiPolylineFeature).flatMap { path in
            path.polylines.count == 1 ? path.polylines[0].trackPoints.map(\.longitude) : nil
        }
    }

    func testWithoutATakeoffTheLineDoesNotStartAtThePlannedHome() {
        XCTAssertEqual(longitudes([home, first, second], false), [44.1, 44.2])
    }

    func testWithATakeoffTheLineStartsAtThePlannedHome() {
        XCTAssertEqual(longitudes([home, first, second], true), [44.0, 44.1, 44.2])
    }

    func testOnePointLeftAfterDroppingHomeIsNoLineAtAll() {
        XCTAssertNil(missionPath([home, first], false))
    }

    func testAnItemWithAnExitIsFlownThroughRatherThanTouched() {
        let survey = MissionItem(index: 1, sequence: 2, latitude: 41.0, longitude: 44.1, command: "Survey", selected: false, altitude: .nan, exit: TrackPoint(latitude: 41.0, longitude: 44.15))
        XCTAssertEqual(longitudes([home, survey, second], false), [44.1, 44.15, 44.2])
    }

    func testAnExitEqualToTheEntryIsNotASecondPoint() {
        XCTAssertEqual(longitudes([home, first, second], false), [44.1, 44.2])
    }

    func testTheRouteDoesNotDetourThroughAStandaloneCoordinate() {
        let roi = MissionItem(index: 2, sequence: 3, latitude: 41.9, longitude: 44.9, command: "ROI", selected: false, altitude: .nan, exit: nil, routed: false)
        XCTAssertEqual(longitudes([home, first, roi, second], false), [44.1, 44.2])
    }

    func testAnItemStillBeingSetUpIsNotLinkedIntoTheRoute() {
        let halfMade = MissionItem(index: 2, sequence: 3, latitude: 41.5, longitude: 44.15, command: "Survey", selected: false, altitude: .nan, exit: nil, routed: false)
        XCTAssertEqual(longitudes([home, first, halfMade, second], false), [44.1, 44.2])
    }

    func testTheCoreDecidesWhetherTheLineStartsAtHomeAndThisHeadNoLongerGuesses() {
        let takeoffButRefused = JSON.parse(#"{"kind":"object","linksStartToHome":false,"items":[{},{"kind":"takeoff"},{}]}"#)
        XCTAssertFalse(
            linksStartToHome(takeoffButRefused),
            "the old rule here was items[1].kind == takeoff, which would say true for this. QGC also suppresses the link when an RTL came earlier, and missionitems.rs walks for that - a head re-deriving from the item list cannot see it"
        )
    }

    func testARoverLinksToHomeWithNoTakeoffItemAnywhere() {
        let rover = JSON.parse(#"{"kind":"object","linksStartToHome":true,"items":[{},{},{}]}"#)
        XCTAssertTrue(
            linksStartToHome(rover),
            "QGC starts the rule at _controllerVehicle->rover(), the OFFLINE editing vehicle, and planningFor never served rover - this head could not have answered it at all"
        )
    }

    func testAnEmptyPlanSaysFalseRatherThanLeavingTheKeyOut() {
        XCTAssertFalse(linksStartToHome(JSON.parse(#"{"kind":"object","linksStartToHome":false,"items":[]}"#)))
        XCTAssertFalse(linksStartToHome(JSON.parse(#"{"kind":"object"}"#)))
        XCTAssertFalse(linksStartToHome(nil))
    }
}
