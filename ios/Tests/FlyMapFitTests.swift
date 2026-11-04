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
