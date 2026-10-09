import XCTest
@testable import Aircast

final class VehicleChangePromptTests: XCTestCase {
    func testThePlanViewAsksAboutADirtyPlanOnlyWhenTheCoreRaisesIt() {
        XCTAssertNil(vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":null}"#)))
        let prompt = vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":{"title":"Plan View - Vehicle Changed","text":"t","loadText":"Load New Plan From Vehicle","keepText":"Keep Current Plan"}}"#))
        XCTAssertEqual("Keep Current Plan", prompt?.keepText)
    }

    func testTheVehicleChangeIsAskedBeforeTheAltitudeAndAltitudeNoIsTheCancel() {
        let both = JSON.parse(#"{"vehicleChangePrompt":{"title":"plan view","text":"t","loadText":"load","keepText":"keep"},"applyAltitudePrompt":{"title":"apply","text":"a"}}"#)
        XCTAssertEqual([LOAD_VEHICLE_PLAN, KEEP_CURRENT_PLAN], corePrompt(both)?.choices.map(\.invoke))
        let altitude = corePrompt(JSON.parse(#"{"vehicleChangePrompt":null,"applyAltitudePrompt":{"title":"apply","text":"a"}}"#))
        XCTAssertEqual([APPLY_DEFAULT_ALTITUDE, DISMISS_ALTITUDE_PROMPT], altitude?.choices.map(\.invoke))
        XCTAssertEqual([false, true], altitude?.choices.map(\.cancel))
        XCTAssertNil(corePrompt(JSON.parse(#"{"vehicleChangePrompt":null,"applyAltitudePrompt":null}"#)))
    }
}
