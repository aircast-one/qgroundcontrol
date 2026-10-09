import XCTest
@testable import Aircast

final class VehicleChangePromptTests: XCTestCase {
    func testThePlanViewAsksOnlyWhenTheCoreRaisesItAndReadsTheAircraftsStoredRoute() {
        XCTAssertNil(vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":null}"#)))
        XCTAssertEqual(
            VehicleChangePrompt(connected: true, dirty: true, aircraftItems: 12),
            vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":{"connected":true,"dirty":true,"aircraftItems":12}}"#))
        )
        XCTAssertNil(vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":{"connected":true,"dirty":true,"aircraftItems":null}}"#))?.aircraftItems)
    }

    func testARouteTheOperatorDrewIsKeptByTheMainButtonAndTheAircraftsRouteIsNamed() {
        let drawn = promptCopy(VehicleChangePrompt(connected: true, dirty: true, aircraftItems: 12))
        XCTAssertEqual(PromptCopy(title: "Aircraft connected", text: "Keep the route you drew, or replace it with the route stored on the aircraft (12 items)?", primary: "Keep my route", secondary: "Load the aircraft's route", primaryKeeps: true), drawn)
        XCTAssertEqual(false, promptCopy(VehicleChangePrompt(connected: true, dirty: false, aircraftItems: nil)).primaryKeeps)
        XCTAssertEqual("Discard it", promptCopy(VehicleChangePrompt(connected: false, dirty: true, aircraftItems: nil)).secondary)
    }

    func testTheVehicleChangeIsAskedBeforeTheAltitudeAndAltitudeNoIsTheCancel() {
        let both = JSON.parse(#"{"vehicleChangePrompt":{"connected":true,"dirty":true},"applyAltitudePrompt":{"title":"apply","text":"a"}}"#)
        XCTAssertEqual([LOAD_VEHICLE_PLAN, KEEP_CURRENT_PLAN], corePrompt(both)?.choices.map(\.invoke))
        XCTAssertEqual([false, true], corePrompt(both)?.choices.map(\.preferred))
        let altitude = corePrompt(JSON.parse(#"{"vehicleChangePrompt":null,"applyAltitudePrompt":{"title":"apply","text":"a"}}"#))
        XCTAssertEqual([APPLY_DEFAULT_ALTITUDE, DISMISS_ALTITUDE_PROMPT], altitude?.choices.map(\.invoke))
        XCTAssertEqual([false, true], altitude?.choices.map(\.cancel))
        XCTAssertNil(corePrompt(JSON.parse(#"{"vehicleChangePrompt":null,"applyAltitudePrompt":null}"#)))
    }
}
