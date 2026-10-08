import XCTest
@testable import Aircast

final class PlanVehicleRowsTests: XCTestCase {
    func testThePlannedVehicleIsChosenOnlyWithNoVehicleAndAnEmptyPlan() {
        XCTAssertTrue(choosesPlanVehicle(false, JSON.parse(#"{"hasMissionItems":false}"#)))
        XCTAssertFalse(choosesPlanVehicle(true, JSON.parse(#"{"hasMissionItems":false}"#)))
        XCTAssertFalse(choosesPlanVehicle(false, JSON.parse(#"{"hasMissionItems":true}"#)))
        XCTAssertTrue(
            choosesPlanVehicle(false, JSON.parse(#"{"containsItems":true,"hasMissionItems":false}"#)),
            "a fence alone leaves the vehicle choosable, as MissionSettingsEditor counts mission items only"
        )
    }
}
