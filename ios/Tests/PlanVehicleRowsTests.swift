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

    func testAnOfflinePlanNamesItsVehicleAsTheVehicleTypeSettingDoes() {
        let named = plannedVehicleRows(JSON.parse(#"{"planningFor":{"firmware":"ArduPilot","type":"Multi-Rotor"}}"#))
        XCTAssertEqual(named.map(\.0), ["Firmware", "Vehicle"])
        XCTAssertEqual(named.map(\.1), ["ArduPilot", "Multi-rotor"])
        XCTAssertEqual(plannedVehicleRows(JSON.parse(#"{"planningFor":{"firmware":"","type":"Quadrotor"}}"#)).map(\.1), ["Quadrotor"])
    }
}
