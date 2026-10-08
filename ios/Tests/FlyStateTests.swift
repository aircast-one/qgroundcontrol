import XCTest
@testable import Aircast

final class FlyStateTests: XCTestCase {
    func testALinkNobodyIsMonitoringIsNotReportedAsHavingContact() {
        let unknown = flyState(JSON.parse(#"{"kind":"object","class":"FlyState","connected":true,"contactLost":null}"#))
        XCTAssertNil(unknown?.contactLost)
        XCTAssertEqual(flyState(JSON.parse(#"{"kind":"object","class":"FlyState","contactLost":false}"#))?.contactLost, false)
        XCTAssertEqual(flyState(JSON.parse(#"{"kind":"object","class":"FlyState","contactLost":true}"#))?.contactLost, true)
    }

    private let inContact = JSON.parse(#"""
        {"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
         "contactLost":false,"state":"disarmed","stateText":"Stabilize · Disarmed",
         "staleNotice":"","mode":"Stabilize"}
        """#)

    private let lost = JSON.parse(#"""
        {"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
         "contactLost":true,"state":"contactLost","stateText":"Communication lost",
         "staleNotice":"No contact — these are the last values the vehicle sent.",
         "mode":"Stabilize"}
        """#)

    func testTheSentenceAndTheTokenComeFromTheCoreNotFromTheHead() throws {
        let state = try XCTUnwrap(flyState(inContact))
        XCTAssertEqual(state.state, "disarmed")
        XCTAssertEqual(state.stateText, "Stabilize · Disarmed")
        XCTAssertEqual(flyState(JSON.parse(#"{"kind":"object","class":"FlyState","connected":true,"stateText":"Ready to Fly"}"#))?.stateText, "Ready to fly")
    }

    func testAVehicleInContactCarriesNoNoticeSoNothingDims() throws {
        let state = try XCTUnwrap(flyState(inContact))
        XCTAssertEqual(state.contactLost, false)
        XCTAssertEqual(state.staleNotice, "")
    }

    func testLostContactCarriesTheNoticeAndTheHeadShowsItVerbatim() throws {
        let state = try XCTUnwrap(flyState(lost))
        XCTAssertEqual(state.contactLost, true)
        XCTAssertEqual(state.stateText, "Communication lost")
        XCTAssertEqual(state.staleNotice, "No contact — these are the last values the vehicle sent.")
    }

    func testTheNoticeUsesTheCoresDashNotAnAsciiHyphen() throws {
        let state = try XCTUnwrap(flyState(lost))
        XCTAssertTrue(state.staleNotice.contains("—"))
        XCTAssertFalse(state.staleNotice.contains(" - "))
    }

    func testAPayloadThatIsNotTheFlyStateYieldsNothing() {
        XCTAssertNil(flyState(nil))
        XCTAssertNil(flyState(JSON.parse("{}")))
        XCTAssertNil(flyState(JSON.parse(#"{"class":"Calibration"}"#)))
    }

    func testAPayloadUsingAnInventedNameForTheNoticeLeavesItBlank() {
        let invented = JSON.parse(#"{"class":"FlyState","contactLost":true,"stale":"No contact","notice":"No contact"}"#)
        XCTAssertEqual(flyState(invented)?.staleNotice, "")
    }
}

final class VehicleSubtitleTests: XCTestCase {
    private func state(connected: Bool = true, contactLost: Bool = false, stateText: String = "Disarmed", mode: String = "Stabilize") -> FlyState {
        FlyState(connected: connected, armed: false, contactLost: contactLost, state: "disarmed", stateText: stateText, staleNotice: "", mode: mode,
                 rcSupported: false, rcSignalText: "", rcSignal: nil, rcOverride: nil, telemetry: nil)
    }

    func testTheHeaderKeepsTheFlightModeTheCoreReportsSeparately() {
        XCTAssertEqual(vehicleSubtitle(state()), "Stabilize · Disarmed")
    }

    func testALostLinkSaysSoOnItsOwnBecauseTheModeIsNoLongerKnown() {
        XCTAssertEqual(vehicleSubtitle(state(contactLost: true, stateText: "Communication lost")), "Communication lost")
    }

    func testNoVehicleSaysSoRatherThanShowingAnEmptyLine() {
        XCTAssertEqual(vehicleSubtitle(nil), "No vehicle")
        XCTAssertEqual(vehicleSubtitle(state(connected: false)), "No vehicle")
    }

    func testWithNoVehicleTheHeaderShowsTheLinkStateLikeMainStatusIndicator() {
        let offline = offlineMainStatus(JSON.parse(#"{"kind":"object","title":"Connection Failed","mainStatus":"Can't Connect"}"#))
        XCTAssertEqual(vehicleSubtitle(state(connected: false), offline), "Can't connect")
        XCTAssertEqual(vehicleSubtitle(nil, offlineMainStatus(JSON.parse(#"{"mainStatus":"Connect a Vehicle"}"#))), "Connect a vehicle")
        XCTAssertEqual(vehicleSubtitle(state(), offline), "Stabilize · Disarmed")
        XCTAssertNil(offlineMainStatus(JSON.parse(#"{"mainStatus":""}"#)))
    }

    func testABlankModeDoesNotLeaveADanglingSeparator() {
        XCTAssertEqual(vehicleSubtitle(state(stateText: "Armed", mode: "")), "Armed")
    }
}

final class FlyStateArmedTests: XCTestCase {
    private func view(armed: Bool, connected: Bool = true) -> FlyState? {
        flyState(JSON.parse(#"""
            {"kind":"object","class":"FlyState","connected":\#(connected),"armed":\#(armed),
             "contactLost":false,"state":"armed","stateText":"Armed","staleNotice":"",
             "mode":"Stabilize","rcSupported":false,"rcSignal":null,"rcSignalText":null}
            """#))
    }

    func testArmedComesFromTheViewRatherThanASecondReadOfTheVehicle() {
        XCTAssertEqual(view(armed: true)?.armed, true)
        XCTAssertEqual(view(armed: false)?.armed, false)
    }

    func testNoVehicleIsNotAnArmedVehicle() {
        XCTAssertEqual(flyState(JSON.parse(#"{"kind":"object","class":"FlyState","connected":false}"#))?.armed, false)
    }

    func testTheStatusSheetSummaryIsTheCoresLine() {
        let state = flyState(JSON.parse(#"{"class":"FlyState","connected":true,"stateText":"Not Fully Ready","summaryDetail":"Mag turned off. Everything else reports normal."}"#))
        XCTAssertEqual(state?.summaryDetail, "Mag turned off. Everything else reports normal.")
        XCTAssertEqual(flyState(JSON.parse(#"{"class":"FlyState","summaryDetail":null}"#))?.summaryDetail, "")
    }
}

final class FlyReadinessTests: XCTestCase {
    private func state(armed: Bool = false, ready: Bool = true, nominal: Bool = true, fault: Bool = false, flying: String = "disarmed") -> FlyState {
        FlyState(connected: true, armed: armed, contactLost: false, state: flying, stateText: ready ? "Ready to fly" : "Not ready", staleNotice: "", mode: "Stabilize",
                 rcSupported: false, rcSignalText: "", rcSignal: nil, rcOverride: nil, telemetry: nil,
                 summaryDetail: "2 checks need attention before arming.", nominal: nominal, fault: fault, readyToFly: ready)
    }

    func testAVehicleThatIsNotReadyIsNotShownGreen() {
        XCTAssertEqual(chipTone(state(ready: false), false), .Neutral)
        XCTAssertEqual(chipTone(state(), false), .Success)
        XCTAssertEqual(chipTone(state(nominal: false), false), .Warning)
        XCTAssertEqual(chipTone(state(fault: true), false), .Error)
        XCTAssertEqual(chipTone(state(armed: true, ready: false), false), .Success)
    }

    func testTheReadinessWarningSaysWhyAndOnlyBeforeArming() {
        XCTAssertEqual(readinessWarning(state(ready: false)), "Not ready. 2 checks need attention before arming.")
        XCTAssertNil(readinessWarning(state()))
        XCTAssertNil(readinessWarning(state(armed: true, ready: false)))
        var passed = state(ready: false)
        passed.summaryDetail = ALL_CHECKS_PASSED
        XCTAssertEqual(readinessWarning(passed), "Not ready. Aircraft setup is not complete.")
    }

    func testAReadinessIssueBlocksOnlyWhenTheVehicleCannotArm() {
        XCTAssertEqual(guidedReadiness(state(ready: false)), Readiness(text: "Not ready. 2 checks need attention before arming.", blocks: false))
        var refusing = state(ready: false)
        refusing.canArm = false
        XCTAssertEqual(guidedReadiness(refusing), Readiness(text: "Not ready. 2 checks need attention before arming. \(READINESS_BLOCKED)", blocks: true))
        XCTAssertNil(guidedReadiness(state()))
    }

    func testDisarmingSaysWhetherTheVehicleFlew() {
        XCTAssertEqual(disarmNotice(true, true, false), "Landed and disarmed")
        XCTAssertEqual(disarmNotice(true, false, false), "Disarmed")
        XCTAssertNil(disarmNotice(false, false, false))
    }

    func testALandingSaysHowLongHowFarAndHowMuchBatteryTheFlightTook() {
        XCTAssertEqual(landedSummary(192, "240 m", 23), "03'12\" \u{00b7} 240 m \u{00b7} 23% battery used")
        XCTAssertEqual(landedSummary(45, nil, nil), "00'45\"")
        XCTAssertEqual(landedSummary(nil, "", 0), "")
        XCTAssertEqual(flownDistanceText(JSON.parse(#"{"kind":"fact","value":53.0,"valueString":"53.0","units":"m"}"#)), "53.0 m")
        XCTAssertNil(flownDistanceText(JSON.parse(#"{"kind":"fact","value":0.0,"valueString":"0.0","units":"m"}"#)))
        XCTAssertNil(flownDistanceText(JSON.parse(#"{"kind":"fact","value":null,"valueString":""}"#)))
    }

    func testTheMapLeadsWhileArmedWithNoVideoSource() {
        XCTAssertEqual(flyViewShown(.Video, true, true), .Map)
        XCTAssertEqual(flyViewShown(.Video, false, true), .Video)
        XCTAssertEqual(flyViewShown(.Video, true, false), .Video)
    }
}
