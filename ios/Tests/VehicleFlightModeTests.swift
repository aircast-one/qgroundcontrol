import XCTest
@testable import Aircast

final class VehicleFlightModeTests: XCTestCase {
    func testEachRowCarriesItsOwnSettableModesAndWritesItsOwnVehicleLikeMultiVehicleListsFlightModeMenu() {
        let choices = vehicleChoices(JSON.parse(#"{"ambiguous":true,"vehicles":[{"id":1,"index":0,"flightModes":["Loiter","RTL"]},{"id":2,"index":1,"flightModes":[]}]}"#)).choices
        XCTAssertEqual(choices[0].flightModes, ["Loiter", "RTL"])
        XCTAssertEqual(vehicleFlightModePath(choices[1]), "vehicles.vehicles.1.flightMode")
        XCTAssertEqual(choices[1].flightModes, [])
    }
}
