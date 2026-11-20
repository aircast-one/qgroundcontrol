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

    func testASilentAircraftMakesNoReadinessClaimsAndTakeOffSaysWhyItCannot() {
        let silentButUnready = JSON.object((lost.object ?? [:]).merging(["nominal": .bool(false), "summaryDetail": .string("GPS turned off. Everything else reports normal.")]) { $1 })
        XCTAssertNil(readinessWarning(flyState(silentButUnready)))
        let noSignal = GuidedOffer(id: "takeoff", title: "Takeoff", offer: "hidden", reason: "Any wording the core chooses.", prompt: "", destructive: false, carriesValue: true, reasonCode: NO_SIGNAL_CODE)
        XCTAssertEqual(takeoffLabel(noSignal, nil), NO_LINK)
        var ready = noSignal
        ready.offer = "ready"
        ready.reason = ""
        ready.reasonCode = ""
        XCTAssertEqual(takeoffLabel(ready, nil), HOLD_TO_TAKE_OFF)
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
        var radio = state(ready: false)
        radio.summaryDetail = "Radio needs setup before this vehicle can fly."
        XCTAssertEqual(readinessWarning(radio), "Not ready. Radio needs setup before this vehicle can fly.", "the core names the setup left, so the warning passes it on")
    }

    func testTheFlyDeckGivesTheReasonWithoutRepeatingTheChip() {
        XCTAssertEqual(readinessWarning(state(ready: false), named: false), "2 checks need attention before arming.")
        var silent = state(ready: false)
        silent.summaryDetail = ""
        XCTAssertEqual(readinessWarning(silent, named: false), "Not ready")
    }

    func testAReadinessIssueBlocksOnlyWhenTheVehicleCannotArm() {
        XCTAssertEqual(guidedReadiness(state(ready: false)), Readiness(text: "Not ready. 2 checks need attention before arming.", blocks: false))
        var refusing = state(ready: false)
        refusing.canArm = false
        XCTAssertEqual(guidedReadiness(refusing), Readiness(text: "Not ready. 2 checks need attention before arming. \(READINESS_BLOCKED)", blocks: true))
        XCTAssertNil(guidedReadiness(state()))
        XCTAssertEqual(guidedReadiness(state(ready: false), blocker: "PreArm: Compass not calibrated")?.text, "Can't arm: Compass not calibrated. Not ready. 2 checks need attention before arming.", "a check the aircraft refused to arm for leads the take-off warning")
        XCTAssertEqual(guidedReadiness(state(), blocker: "PreArm: Compass not calibrated"), Readiness(text: "Can't arm: Compass not calibrated.", blocks: false), "the refusal alone is reason enough to warn")
        XCTAssertNil(guidedReadiness(state(armed: true), blocker: "PreArm: Compass not calibrated"))
        XCTAssertNil(guidedReadiness(state(), blocker: "A sensor is reporting unhealthy. The vehicle will refuse to arm."), "a guess the status line already makes is not a refusal")
    }

    func testDisarmingSaysWhetherTheVehicleFlew() {
        XCTAssertEqual(disarmNotice(true, true, false), "Landed and disarmed")
        XCTAssertEqual(disarmNotice(true, false, false), "Disarmed")
        XCTAssertNil(disarmNotice(false, false, false))
        XCTAssertEqual(disarmNotice(true, true, false, DisarmCause(killed: true, lands: true)), "Motors stopped", "killed in the air it fell, it did not land")
        XCTAssertEqual(disarmNotice(true, true, false, DisarmCause(killed: true, lands: false)), "Motors stopped", "a sub killed at depth")
        XCTAssertEqual(disarmNotice(true, true, false, DisarmCause(killed: false, lands: false)), "Disarmed", "a sub disarmed at the surface")
    }

    func testAnArmedFlightOutlivesALinkLossALandingDuringTheOutageStillReadsAsLandedAndTheStartIsNotReRead() {
        let flying = armedSessionStep(armedSessionStep(NO_ARMED_SESSION, true, 1, true, "disarmed").session, true, 1, true, "flying").session
        XCTAssertEqual(flying, ArmedSession(vehicle: 1, armed: true, flew: true))
        let outage = armedSessionStep(flying, false, nil, false, nil)
        XCTAssertEqual(outage, ArmedSessionStep(session: flying, notice: nil, landed: false, started: false))
        XCTAssertEqual(armedSessionStep(outage.session, true, 1, false, "landed"), ArmedSessionStep(session: ArmedSession(vehicle: 1, armed: false, flew: false), notice: "Landed and disarmed", landed: true, started: false))
        XCTAssertEqual(armedSessionStep(outage.session, true, 1, true, "landed"), ArmedSessionStep(session: flying, notice: nil, landed: true, started: false), "still armed on its return, the flight carries on")
        XCTAssertEqual(armedSessionStep(outage.session, true, nil, false, "disarmed"), ArmedSessionStep(session: flying, notice: nil, landed: false, started: false), "nothing is said until the vehicle is named")
        XCTAssertEqual(armedSessionStep(flying, true, 1, false, "disarmed", DisarmCause(killed: true, lands: true)).notice, MOTORS_STOPPED)
    }

    func testADifferentAircraftOnTheLostOnesIdStartsAfreshAndAnnouncesNothingAboutTheLostOne() {
        let flying = armedSessionStep(armedSessionStep(NO_ARMED_SESSION, true, 1, true, "flying").session, false, nil, false, nil).session
        XCTAssertEqual(armedSessionStep(flying, true, 2, false, "disarmed"), ArmedSessionStep(session: ArmedSession(vehicle: 2, armed: false, flew: false), notice: nil, landed: false, started: false))
        XCTAssertTrue(armedSessionStep(flying, true, 2, true, "disarmed").started)
        XCTAssertEqual(flyState(JSON.parse(#"{"class":"FlyState","connected":true,"instance":2}"#))?.instance, 2)
        XCTAssertNil(flyState(JSON.parse(#"{"class":"FlyState","connected":true}"#))?.instance)
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
