import XCTest
@testable import Aircast

final class FlyMapFitTests: XCTestCase {
    func testTheFlyMapFitsWheneverANewMissionArrivesFromTheVehicleAsOnNewItemsFromVehicleDoes() {
        let same = { (it: Int) in it }
        XCTAssertTrue(missionArrived([], [1, 2], same))
        XCTAssertTrue(missionArrived([1], [1, 2], same), "a replaced mission refits, as onNewItemsFromVehicle fits every download with items")
        XCTAssertFalse(missionArrived([1, 2], [1, 2], same), "an unchanged mission does not refit and fight the pilot's pan")
        XCTAssertFalse(missionArrived([1], [], same))
    }

    func testTheMapCentresOnceOnTheOperatorWhenNoVehicleHasAPositionAsFlightMapDoesOnTheFirstGcsFix() {
        let here = TrackPoint(latitude: 47.0, longitude: 8.0)
        XCTAssertTrue(centresOnOperator(false, here, false))
        XCTAssertFalse(centresOnOperator(true, here, false))
        XCTAssertFalse(centresOnOperator(false, here, true))
        XCTAssertFalse(centresOnOperator(false, nil, false))
    }

    func testAMapStillShowingTheWholeWorldOpensOnTheHomeElseTheLaunchOfTheFlownMissionElseNothingItDoesNotKnow() {
        let home = TrackPoint(latitude: 41.72, longitude: 44.88)
        let launch = TrackPoint(latitude: 41.7, longitude: 44.8)
        let items = [
            MissionItem(index: 0, sequence: 0, latitude: launch.latitude, longitude: launch.longitude, command: "Launch", selected: false),
            MissionItem(index: 1, sequence: 1, latitude: 41.8, longitude: 44.9, command: "Waypoint", selected: false),
        ]
        XCTAssertTrue(showsWholeWorld(0))
        XCTAssertFalse(showsWholeWorld(15))
        XCTAssertEqual(launchOf(items), launch)
        XCTAssertEqual(firstKnown([home, launch]), home)
        XCTAssertEqual(firstKnown([nil, TrackPoint(latitude: 0, longitude: 0), launch]), launch, "an unset home reads 0, 0, which is no place")
        XCTAssertNil(firstKnown([nil, nil]))
        XCTAssertNil(launchOf(Array(items.dropFirst())))
    }

    func testTheMapGoesToTheHomeOrLaunchOnceOnlyWhenItOpenedOnTheWholeWorldWithNoRememberedViewAndNoAircraftOnIt() {
        XCTAssertTrue(centresOnLaunch(false, false, 0.4, false))
        XCTAssertFalse(centresOnLaunch(false, false, nil, false), "a map that has not reported its zoom yet is not known to show the whole world")
        XCTAssertFalse(centresOnLaunch(false, true, 0.4, false), "a remembered view goes first")
        XCTAssertFalse(centresOnLaunch(true, false, 0.4, false), "zooming back out to the world later does not snap to the launch again")
        XCTAssertFalse(centresOnLaunch(false, false, 0.4, true), "an aircraft with a position is followed instead")
        XCTAssertFalse(centresOnLaunch(false, false, 12, false))
    }

    func testOtherVehiclesMissionsComeFromTheFlyViewAsFlyViewMapRepeatsPlanMapItemsForEveryVehicle() {
        let view = JSON.parse(#"{"items":[],"others":[{"linksStartToHome":true,"items":[]},{"items":[]}]}"#)
        XCTAssertEqual(otherMissions(view).map(\.linkStartToHome), [true, false])
        XCTAssertEqual(otherMissions(nil), [])
    }

    func testAGotoPreviewFitsTheAircraftAndThePendingTarget() {
        let target = TrackPoint(latitude: 47.398, longitude: 8.546)
        let aircraft = TrackPoint(latitude: 47.397, longitude: 8.545)
        XCTAssertEqual(gotoPreviewFit(aircraft, target), [aircraft, target])
        XCTAssertEqual(gotoPreviewFit(nil, target), [target])
        XCTAssertNil(gotoPreviewFit(aircraft, nil), "without a preview the map fits the plan as usual")
    }
}
