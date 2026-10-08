import XCTest
@testable import Aircast

final class SetupViewTests: XCTestCase {
    func testSetupWordingFollowsVehicleSummaryAndSetupView() {
        XCTAssertEqual(setupReadiness(JSON.parse(#"{"ready":false,"setupComplete":true}"#))?.setupComplete, true)
        XCTAssertNil(setupReadiness(JSON.parse(#"{"ready":false}"#))?.setupComplete)
    }

    func testACoreThatDeclinesToJudgeIsNotJudgedAsNotReady() {
        XCTAssertNil(
            setupReadiness(JSON.parse(#"{"ready":null,"headline":"No vehicle connected"}"#))?.ready,
            "readiness() answers Option<bool> and returns None when it has no basis for a verdict - "
                + "its own test says the false that means \"checked and not ready\" once drew an error "
                + "heading beside an instruction to connect a vehicle. optBoolean flattens JSON null "
                + "to false, so the head could not hold that third state at all"
        )
    }

    func testAVerdictThatWasReachedIsStillCarried() {
        XCTAssertEqual(setupReadiness(JSON.parse(#"{"ready":true}"#))?.ready, true)
        XCTAssertEqual(setupReadiness(JSON.parse(#"{"ready":false}"#))?.ready, false)
    }

    private let corePages = JSON.parse(
        #"""
        {"groups":[
            {"title":"Vehicle","pages":[{"name":"Summary","native":true}]},
            {"title":"Setup","pages":[
                {"name":"Sensors","native":true},{"name":"Radio","native":true},
                {"name":"Frame","native":true},{"name":"Flight Modes","native":true},
                {"name":"Safety","native":true},{"name":"Power","native":true},
                {"name":"Motors","native":true},{"name":"Tuning","native":true},
                {"name":"Camera","native":true},{"name":"Lights","native":true},
                {"name":"Flight Behavior","native":false}]},
            {"title":"Advanced","pages":[
                {"name":"Remote Support","native":true},{"name":"Parameters","native":true}]}]}
        """#
    )

    func testEveryPageTheHeadNamesIsAPageTheCoreOffers() {
        let offered = Set(setupGroups(corePages).flatMap(\.pages).map(\.name))
        let claimed: Set<String> = [RADIO, SENSORS, "Remote Support"]
        XCTAssertEqual(claimed.subtracting(offered), [])
    }

    func testAPageOpensOnWhatThisHeadHasNotOnAFieldTheCoreMayStopServing() {
        let served = JSON.parse(#"{"groups":[{"title":"S","pages":[{"name":"Radio","parameterSections":false}]}]}"#)
        let page = setupPage(served, RADIO)
        XCTAssertTrue(headCanOpen(page, RADIO), "Radio has a screen here, so no flag from the core decides it")
    }

    func testAPageTheHeadHasNoScreenForDoesNotOpen() {
        let summary = SetupPage(name: "Summary", parameterSections: false)
        let frame = SetupPage(name: "Frame", parameterSections: false)
        XCTAssertFalse(
            headCanOpen(summary, "Summary"),
            "an unnamed page once opened the Remote Support screen, which forwards live position to a third party"
        )
        XCTAssertFalse(headCanOpen(frame, "Frame"))
    }

    func testEveryNativeScreenTheSetupPageRoutesToAlsoOpensFromTheList() {
        XCTAssertTrue(headCanOpen(SetupPage(name: "Frame", parameterSections: false, screen: APM_AIRFRAME_SCREEN), "Frame"))
        XCTAssertTrue(headCanOpen(SetupPage(name: "Syslink", parameterSections: false, screen: SYSLINK_SCREEN), "Syslink"))
        XCTAssertTrue(headCanOpen(SetupPage(name: "Failsafes", parameterSections: false, screen: NOT_SUPPORTED_SCREEN), "Failsafes"))
    }

    func testMotorsOpensHereNowBecauseTheOperatorAskedForItRatherThanTheDesktop() {
        XCTAssertTrue(headCanOpen(SetupPage(name: "Motors", parameterSections: false), MOTORS))
    }

    func testTheScreensTheHeadDoesHaveStillOpen() {
        XCTAssertTrue(headCanOpen(SetupPage(name: SENSORS, parameterSections: false), SENSORS))
        XCTAssertTrue(headCanOpen(SetupPage(name: RADIO, parameterSections: false), RADIO))
        XCTAssertTrue(headCanOpen(SetupPage(name: REMOTE_SUPPORT, parameterSections: false), REMOTE_SUPPORT))
        XCTAssertTrue(headCanOpen(SetupPage(name: "Safety", parameterSections: true), "Safety"))
    }

    func testAPageTheCoreDescribesAsParametersOpensAndNoPageOpensWithoutOne() {
        XCTAssertTrue(headCanOpen(SetupPage(name: "Tuning", parameterSections: true), "Tuning"))
        XCTAssertFalse(headCanOpen(nil, SENSORS))
    }
}

final class SetupReadinessTests: XCTestCase {
    func testNoViewMeansNoReadiness() {
        XCTAssertNil(setupReadiness(nil))
    }

    func testReadinessComesFromTheCoreVerbatim() {
        let readiness = setupReadiness(JSON.parse(#"{"ready":false,"headline":"2 components need setup","detail":"A sensor is unhealthy."}"#))
        XCTAssertEqual(readiness?.ready, false)
        XCTAssertEqual(readiness?.headline, "2 components need setup")
        XCTAssertEqual(readiness?.detail, "A sensor is unhealthy.")
    }

    func testAReadyVehicleCarriesNoHeadline() {
        let readiness = setupReadiness(JSON.parse(#"{"ready":true,"headline":"","detail":""}"#))
        XCTAssertEqual(readiness?.ready, true)
        XCTAssertEqual(readiness?.headline, "")
    }
}

final class SetupFirmwareTests: XCTestCase {
    func testTheServedFirmwareTokenAnswersPx4ApmAndNoVehicleApart() {
        let px4 = setupReadiness(JSON.parse(#"{"connected":true,"firmware":"px4"}"#))
        let apm = setupReadiness(JSON.parse(#"{"connected":true,"firmware":"apm"}"#))
        let none = setupReadiness(JSON.parse(#"{"connected":false,"firmware":"none"}"#))
        XCTAssertTrue(isPx4(px4))
        XCTAssertFalse(isPx4(apm), "apm is not px4")
        XCTAssertFalse(isPx4(none), "and no vehicle is not px4 either, which a boolean could not distinguish")
        XCTAssertFalse(isPx4(nil))
    }

    func testConnectedComesFromTheSameViewRatherThanASecondRead() {
        XCTAssertTrue(setupReadiness(JSON.parse(#"{"connected":true}"#))!.connected)
        XCTAssertFalse(setupReadiness(JSON.parse(#"{"connected":false}"#))!.connected)
    }

    func testAComponentNeedingAttentionNamesWhatToDoCalibratingSensorsAndRadio() {
        XCTAssertEqual(attentionAction("APMRadioComponent"), "Calibrate")
        XCTAssertEqual(attentionAction("SensorsComponent"), "Calibrate")
        XCTAssertEqual(attentionAction("APMPowerComponent"), "Set up")
    }

    func testTheHeaderNamesFirmwareThenFrameAndSpeaksOnlyWhenTheListCannot() {
        XCTAssertEqual(setupSubtitle("Quadrotor", "ArduPilot 4.5.7"), "ArduPilot 4.5.7 · Quadrotor")
        let pending = SetupReadiness(ready: false, setupComplete: false, headline: "1 component needs setup", detail: "Radio", connected: true, firmware: "")
        let complete = SetupReadiness(ready: false, setupComplete: true, headline: "1 component needs setup", detail: "Radio", connected: true, firmware: "")
        XCTAssertNil(readinessNote(pending, true))
        XCTAssertEqual(readinessNote(pending, false), "1 component needs setup. Radio")
        XCTAssertNil(readinessNote(complete, false))
    }

    func testTheParametersRowCountsWhatTheVehicleHasAsPenpot() {
        XCTAssertEqual(parameterCountText(1204), "1,204 parameters")
        XCTAssertNil(parameterCountText(0))
    }
}
