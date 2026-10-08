import XCTest
@testable import Aircast

final class VehicleChangePromptTests: XCTestCase {
    func testThePlanViewAsksAboutADirtyPlanOnlyWhenTheCoreRaisesIt() {
        XCTAssertNil(vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":null}"#)))
        let prompt = vehicleChangePrompt(JSON.parse(#"{"vehicleChangePrompt":{"title":"Plan View - Vehicle Changed","text":"t","loadText":"Load New Plan From Vehicle","keepText":"Keep Current Plan"}}"#))
        XCTAssertEqual("Keep Current Plan", prompt?.keepText)
    }
}
