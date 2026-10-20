import XCTest
@testable import Aircast

final class VehicleStatusSheetTests: XCTestCase {
    func testTheStatusSheetListsFaultySensorsUntilAskedForTheRest() {
        let sensors = [SensorHealth(name: "Gyro", state: "unhealthy", label: "Error"), SensorHealth(name: "Baro", state: "healthy", label: "Normal")]
        XCTAssertEqual(shownSensors(sensors, false).map(\.name), ["Gyro"])
        XCTAssertEqual(shownSensors(sensors, true).map(\.name), ["Gyro", "Baro"])
    }

    private func state(armed: Bool = false, nominal: Bool = true, fault: Bool = false, canArm: Bool = true) -> FlyState {
        FlyState(
            connected: true, armed: armed, contactLost: false, state: "", stateText: "", staleNotice: "", mode: "",
            rcSupported: false, rcSignalText: "", rcSignal: nil, rcOverride: nil, telemetry: nil,
            nominal: nominal, fault: fault, canArm: canArm
        )
    }

    func testTheMessagesCardStartsCollapsedAndTogglesLikeTheMainStatusIndicatorDisclosure() {
        XCTAssertEqual(messagesToggleText(false), "Show messages")
        XCTAssertEqual(messagesToggleText(true), "Hide messages")
    }

    func testTheStatusDrawersArmControlsFollowMainStatusIndicator() {
        XCTAssertEqual(armControls(state(), false), ArmControls(holdText: "Slide to arm", holdEnabled: true, mayBeRefused: false, forceLink: false, forceHold: false))
        XCTAssertEqual(armControls(state(armed: true), false), ArmControls(holdText: "Slide to disarm", holdEnabled: true, mayBeRefused: false, forceLink: false, forceHold: false))
        XCTAssertEqual(armControls(state(nominal: false), false), ArmControls(holdText: "Slide to arm", holdEnabled: true, mayBeRefused: true, forceLink: false, forceHold: false))
        XCTAssertEqual(armControls(state(nominal: false, fault: true, canArm: false), false), ArmControls(holdText: "Slide to arm", holdEnabled: false, mayBeRefused: false, forceLink: true, forceHold: false))
        XCTAssertEqual(armControls(state(canArm: false), true), ArmControls(holdText: "Slide to arm", holdEnabled: false, mayBeRefused: false, forceLink: false, forceHold: true))
    }

    func testASlideTheDeckCannotActOnSaysWhyInsteadOfClosingSilently() {
        let blocked = GuidedOffer(id: "arm", title: "Arm", offer: "disabled", reason: "Complete the preflight checklist first", prompt: "", destructive: false, carriesValue: false)
        XCTAssertEqual(deckRequestRefusal(blocked), "Complete the preflight checklist first")
        XCTAssertEqual(deckRequestRefusal(nil), ARM_UNAVAILABLE)
    }

    func testACheckWithADescriptionTogglesOpenAndShutOneWithoutStaysShutLikeTheOverallStatusDelegate() {
        let detailed = ArmingCheck(message: "GPS not ready", description: "Wait for a fix", severity: "error")
        let bare = ArmingCheck(message: "Battery low", description: "", severity: "warning")
        XCTAssertEqual(expandedAfterTap([], 0, detailed), [0])
        XCTAssertEqual(expandedAfterTap([0], 0, detailed), [])
        XCTAssertEqual(expandedAfterTap([], 1, bare), [])
    }

    func testAStatusLineThatNamesAPayloadProblemCarriesTheOneTapFixThePayloadPanelOffersForIt() {
        let open = "The gripper stays open after the first drop, so the other 3 drops release nothing. Turn on auto-close so it closes after each drop."
        let view = { (problem: String) in
            flyState(JSON.parse(#"{"class":"FlyState","connected":true,"stateText":"Not Fully Ready","summaryDetail":"\#(open)","summaryProblem":\#(problem)}"#))
        }
        XCTAssertEqual(
            statusSummary(view(#"{"code":"staysOpen","text":"\#(open)","blocking":true,"fix":"Turn on auto-close"}"#))?.problem,
            PayloadProblem(code: "staysOpen", text: open, blocking: true, fix: "Turn on auto-close")
        )
        XCTAssertEqual(statusSummary(view("null")), StatusSummaryLines(title: "Not fully ready", detail: open, problem: nil))
        XCTAssertNil(statusSummary(flyState(JSON.parse(#"{"class":"FlyState","summaryDetail":""}"#))))
    }
}
