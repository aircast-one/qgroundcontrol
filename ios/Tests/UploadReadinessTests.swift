import XCTest
@testable import Aircast

final class UploadReadinessTests: XCTestCase {
    private func view(_ ready: Bool, _ reason: String, canSend: Bool = true) -> JSON {
        JSON.parse(#"{"readiness":{"ready":\#(ready),"reason":"\#(reason)"},"upload":{"canSend":\#(canSend),"canProceed":false,"pausesFirst":false,"heading":"","refusal":"","proceedTitle":""}}"#)
    }

    func testAPlanWithAnItemStillBeingDrawnIsNotSent() {
        let v = view(false, "An item is still being drawn, so the plan cannot be saved or sent.")
        XCTAssertEqual(
            uploadStep(uploadGate(v), notReady: notReadyToSend(v)),
            .Refuse("An item is still being drawn, so the plan cannot be saved or sent.")
        )
    }

    func testReadinessIsCheckedBeforeTheVehiclePrecheckBecauseItIsAboutThePlan() {
        let v = view(false, "Waiting for terrain heights before the plan can be saved or sent.", canSend: true)
        guard case .Refuse = uploadStep(uploadGate(v), notReady: notReadyToSend(v)) else { return XCTFail("expected a refusal") }
    }

    func testAReadyPlanIsSentAsBefore() {
        let v = view(true, "")
        XCTAssertNil(notReadyToSend(v))
        XCTAssertEqual(uploadStep(uploadGate(v), notReady: notReadyToSend(v)), .Send)
    }

    func testAViewWithNoReadinessAtAllDoesNotBlockTheUpload() {
        XCTAssertNil(notReadyToSend(JSON.parse(#"{"upload":{"canSend":true}}"#)))
        XCTAssertNil(notReadyToSend(nil))
    }

    func testAnUnreadyPlanWithNoReasonStillSaysSomething() {
        XCTAssertEqual(notReadyToSend(view(false, "")), "The plan is not ready to send.")
    }

    func testARefusedUploadGoesToTheFirstItemThatStillNeedsSomething() {
        XCTAssertEqual(nextNotReady(JSON.parse(#"{"readiness":{"ready":false,"next":3}}"#)), 3)
        XCTAssertNil(nextNotReady(JSON.parse(#"{"readiness":{"ready":false,"next":null}}"#)))
        XCTAssertNil(nextNotReady(JSON.parse(#"{"readiness":{"ready":true,"next":3}}"#)))
    }
}
