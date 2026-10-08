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
}
