import XCTest
@testable import Aircast

final class PlanSupportTests: XCTestCase {
    private func view(_ json: String) -> PlanSupport { planSupport(JSON.parse(json)) }

    func testAVehicleThatTakesBothOffersBoth() {
        let both = view(#"{"kind":"object","actions":{"addFence":true,"addRally":true},"fenceSupported":true,"rallySupported":true}"#)
        XCTAssertTrue(both.fence)
        XCTAssertTrue(both.rally)
        XCTAssertFalse(both.fenceRefused)
    }

    func testAVehicleThatTakesNeitherSaysSoInsteadOfOfferingButtonsThatFail() {
        let neither = view(#"{"kind":"object","actions":{"addFence":false,"addRally":false},"fenceSupported":false,"rallySupported":false}"#)
        XCTAssertFalse(neither.fence)
        XCTAssertFalse(neither.rally)
        XCTAssertTrue(neither.fenceRefused)
        XCTAssertTrue(neither.rallyRefused)
    }

    func testTheTwoAreAnsweredSeparatelyBecauseAVehicleCanTakeOneAndNotTheOther() {
        let fenceOnly = view(#"{"kind":"object","actions":{"addFence":true,"addRally":false}}"#)
        XCTAssertTrue(fenceOnly.fence)
        XCTAssertFalse(fenceOnly.rally)
    }

    func testNoAnswerHidesTheButtonsRatherThanOfferingAnActionTheVehicleWillRefuse() {
        XCTAssertFalse(planSupport(nil).fence)
        XCTAssertFalse(planSupport(nil).rally)
        XCTAssertFalse(view(#"{"kind":"null"}"#).fence)
    }

    func testOnlyAnExplicitRefusalHidesTheEmptyLayerHintLikeRallyPointEditorAndGeoFenceEditor() {
        let rallyOnlyRefused = view(#"{"kind":"object","actions":{"addFence":true,"addRally":false},"fenceSupported":true,"rallySupported":false}"#)
        let unread = view(#"{"kind":"object","actions":{"addFence":false,"addRally":false}}"#)
        XCTAssertTrue(rallyOnlyRefused.rallyRefused)
        XCTAssertFalse(rallyOnlyRefused.fenceRefused)
        XCTAssertFalse(unread.rallyRefused)
        XCTAssertFalse(unread.fenceRefused)
    }
}
