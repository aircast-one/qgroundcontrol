import XCTest
@testable import Aircast

final class SetupScreenTests: XCTestCase {
    func testTheFirmwareLineIsTheOneTheCoreSpelled() {
        let line = firmwareLine(JSON.parse(#"{"kind":"object","available":true,"summary":"PX4 Pro 1.15.0 beta","vehicleType":"Quadrotor"}"#))
        XCTAssertEqual(line, FirmwareLine(summary: "PX4 Pro 1.15.0 beta", vehicleType: "Quadrotor"))
    }

    func testTheScreenReportsOpeningUnlessAParameterFormDoesItAndAlwaysWhileAPrerequisiteBlocksThePage() {
        XCTAssertTrue(reportsOpening(nil, SetupPage(name: "Sensors", parameterSections: false)))
        XCTAssertFalse(reportsOpening(nil, SetupPage(name: "Heli", parameterSections: true)))
        XCTAssertTrue(reportsOpening("Radio", SetupPage(name: "Flight Modes", parameterSections: false)))
        XCTAssertTrue(reportsOpening("Airframe", SetupPage(name: "Power", parameterSections: true)))
        XCTAssertFalse(reportsOpening(nil, SetupPage(name: "Flight Safety", parameterSections: false, screen: NOT_SUPPORTED_SCREEN)))
    }

    func testParametersReadReadyOnlyOnAConnectedVehicleThatSaysSo() {
        XCTAssertTrue(parametersReady(JSON.parse(#"{"connected":true,"parametersReady":true}"#)))
        XCTAssertFalse(parametersReady(JSON.parse(#"{"connected":false,"parametersReady":true}"#)))
        XCTAssertFalse(parametersReady(JSON.parse(#"{"connected":true,"parametersReady":false}"#)))
        XCTAssertFalse(parametersReady(nil))
    }

    func testNoVehicleReadsAsEmptyRatherThanStraySeparators() {
        XCTAssertEqual(firmwareLine(nil), FirmwareLine(summary: "", vehicleType: ""))
        XCTAssertEqual(firmwareLine(JSON.parse(#"{"kind":"object","available":false,"summary":"","vehicleType":""}"#)), FirmwareLine(summary: "", vehicleType: ""))
    }

    func testAComponentIsReadWithTheVerdictsTheCoreReached() {
        let view = JSON.parse(
            #"{"components":[{"name":"Radio","needsAttention":true,"blockedReason":"armed"},"#
                + #"{"name":"Camera","needsAttention":false,"blockedReason":null}]}"#
        )
        let read = setupComponents(view)

        XCTAssertEqual(read.map(\.name), ["Radio", "Camera"])
        XCTAssertEqual(read.map(\.needsAttention), [true, false])
        XCTAssertEqual(read.map(\.blockedReason), ["armed", nil])
    }

    func testABlockedPageSaysWhyLikeSetupPage() {
        XCTAssertEqual(disabledWhile("flying"), "Disabled while the vehicle is flying")
    }

    func testAComponentWithNoNameIsNotOfferedAndNoViewIsNoComponents() {
        XCTAssertEqual(setupComponents(JSON.parse(#"{"components":[{"name":"Radio"},{}]}"#)).count, 1)
        XCTAssertEqual(setupComponents(nil).map(\.name), [])
    }

    func testAComponentPromotedForAttentionIsNotListedASecondTime() {
        let components = [
            SetupComponent(index: 0, name: "Frame", needsAttention: false),
            SetupComponent(index: 1, name: "Sensors", needsAttention: true),
            SetupComponent(index: 2, name: "Power", needsAttention: false),
        ]

        XCTAssertEqual(remainingSetup(components).map(\.name), ["Frame", "Power"])
    }

    func testEveryComponentNeedingAttentionLeavesNothingForTheFullList() {
        let components = [SetupComponent(index: 0, name: "Sensors", needsAttention: true)]

        XCTAssertEqual(remainingSetup(components).map(\.name), [])
    }

    func testAParameterLoadThatHasStoppedIsNotDrawnAsOneStillRunning() {
        let view = { (reason: String, ready: Bool) in
            JSON.parse(
                #"{"parametersReady":\#(ready),"parametersReason":"\#(reason)","#
                    + #""parametersText":"This vehicle has not answered the request for its parameters, and the retries are finished."}"#
            )
        }

        XCTAssertEqual(parameterWait(view("loading", false))?.title, "Loading parameters from the vehicle.")
        XCTAssertEqual(parameterWait(view("loading", false))?.body, "")

        let stopped = parameterWait(view("unanswered", false))
        XCTAssertEqual(
            stopped?.title,
            "This vehicle has not answered the request for its parameters, and the retries are finished.",
            "the core states what the vehicle did"
        )
        XCTAssertEqual(
            stopped?.body,
            "Setup needs them. Disconnect and connect the link to ask again.",
            "and this head owns the only thing an operator can act on from here"
        )

        XCTAssertNil(parameterWait(view("", true)), "a ready vehicle waits for nothing")
        XCTAssertNil(parameterWait(view("noVehicle", false)))
        XCTAssertNil(parameterWait(nil), "the screen already says to connect a vehicle")
    }

    func testAReasonThisHeadHasNeverHeardOfReadsAsStoppedNotAsReady() {
        let future = JSON.parse(#"{"parametersReady":false,"parametersReason":"refused","parametersText":"The vehicle refused the request."}"#)
        let waiting = parameterWait(future)
        XCTAssertEqual(waiting?.title, "The vehicle refused the request.")
        XCTAssertEqual(
            waiting?.body,
            PARAMETERS_STOPPED,
            "ready would hide a dead load behind a normal screen and loading is the defect being fixed, so an unfamiliar state is closer to stopped than to either"
        )
    }

    func testAReasonWithNoSentenceBesideItStillSaysSomethingTrue() {
        let bare = JSON.parse(#"{"parametersReady":false,"parametersReason":"refused","parametersText":""}"#)
        XCTAssertEqual(parameterWait(bare)?.title, "This vehicle has not sent its parameters (refused).")
    }

    func testADownloadSkippedInFlightOffersToFetchTheParameters() {
        let wait = parameterWait(JSON.parse(#"{"parametersReady":false,"parametersReason":"skipped","parametersText":"Parameter download was skipped because the vehicle is flying."}"#))
        XCTAssertEqual(wait?.downloadOffered, true)
        XCTAssertEqual(wait?.title, "Parameter download was skipped because the vehicle is flying.")
    }

    func testAComponentWithAnUnfinishedPrerequisiteSaysWhichOne() {
        let view = JSON.parse(#"{"components":[{"name":"Flight Modes","needsAttention":true,"blockedReason":null,"prerequisite":"Frame"}]}"#)
        XCTAssertEqual(setupComponents(view).first?.prerequisite, "Frame")
        XCTAssertEqual(prerequisiteText("Frame", "Flight Modes"), "Frame has to be set up before Flight modes.")
    }

    func testTheSetupSearchNarrowsPagesByNameIgnoringCaseAndSpaces() {
        XCTAssertTrue(setupMatches("Flight Modes", "  flight "))
        XCTAssertTrue(setupMatches("Radio", ""))
        XCTAssertFalse(setupMatches("Radio", "sensor"))
    }

    func testAVehicleThatWithheldParametersReadsAsSetupViewsParametersIncomplete() {
        let view = JSON.parse(#"{"parametersReady":true,"parametersReason":"incomplete","parametersText":"The vehicle didn't return its full parameter list, so some setup options are unavailable."}"#)
        XCTAssertEqual(parametersIncomplete(view), "Parameters Incomplete. The vehicle didn't return its full parameter list, so some setup options are unavailable.")
        XCTAssertNil(parametersIncomplete(JSON.parse(#"{"parametersReady":true,"parametersReason":""}"#)))
        XCTAssertNil(parameterWait(view), "ready parameters never block setup")
    }
}
