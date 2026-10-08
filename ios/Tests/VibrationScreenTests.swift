import XCTest
@testable import Aircast

final class VibrationScreenTests: XCTestCase {
    private let served = #"""
        {"available":true,"units":"m/s²","scaleMaximum":90,"warningLevel":30,"dangerLevel":60,
         "axes":[
           {"axis":"X","value":15.0,"fraction":0.1667,"severity":"normal"},
           {"axis":"Y","value":45.0,"fraction":0.5,"severity":"warning"},
           {"axis":"Z","value":75.0,"fraction":0.8333,"severity":"danger"}],
         "clipCounts":[0,3,12],"worst":"danger","clipping":true}
        """#

    func testEachSeverityKeepsTheWordThisScreenHasAlwaysShown() {
        XCTAssertEqual(severityLabel("normal"), "Healthy")
        XCTAssertEqual(severityLabel("warning"), "Watch")
        XCTAssertEqual(severityLabel("danger"), "Unsafe")
    }

    func testAnAbsentSeverityIsBlankRatherThanAGuess() {
        XCTAssertEqual(severityLabel(nil), "")
        XCTAssertEqual(severityLabel("something the core added later"), "")
    }

    func testTheReadingCarriesTheAxesAndClipCountsTheCoreServed() throws {
        let reading = try XCTUnwrap(vibrationReading(JSON.parse(served)))

        XCTAssertEqual(reading.axes.map(\.axis), ["X", "Y", "Z"])
        XCTAssertEqual(reading.axes.map(\.value), [15.0, 45.0, 75.0])
        XCTAssertEqual(reading.axes.map(\.severity), ["normal", "warning", "danger"])
        XCTAssertEqual(reading.clipCounts, [0, 3, 12])
    }

    func testANullAxisReadsAsAbsentNotAsZeroAtTheBottomOfTheScale() throws {
        let reading = try XCTUnwrap(vibrationReading(JSON.parse(#"""
            {"available":true,"units":"m/s²","scaleMaximum":90,
             "warningLevel":30,"dangerLevel":60,
             "axes":[{"axis":"X","value":null,"fraction":null,"severity":null}],
             "clipCounts":[]}
            """#)))

        XCTAssertNil(reading.axes[0].value)
        XCTAssertNil(reading.axes[0].severity)
        XCTAssertEqual(reading.axes[0].fraction, 0, accuracy: 1e-6)
    }

    func testAnUnavailableViewIsNoReadingAtAll() {
        XCTAssertNil(vibrationReading(nil))
        XCTAssertNil(vibrationReading(JSON.parse(#"{"available":false}"#)))
    }

    func testAHealthyThreeAxisReadingDrawsTheBarsRatherThanAnEmptyState() {
        let healthy = JSON.parse(#"""
            {"kind":"object","class":"Vibration","connected":true,"available":true,
             "silentReason":null,"silentText":null,"units":"m/s^2","scaleMaximum":90,
             "warningLevel":30,"dangerLevel":60,"clipCounts":[0,0,0],
             "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                     {"axis":"y","label":"Y","value":14.0,"fraction":0.15,"severity":"normal"},
                     {"axis":"z","label":"Z","value":16.0,"fraction":0.17,"severity":"normal"}]}
            """#)

        XCTAssertNotNil(vibrationReading(healthy))
        XCTAssertNil(vibrationEmptyState(healthy, vibrationReading(healthy)), "an empty state here means the screen never draws its bars at all")
    }

    func testSomeAxesReportedAndSomeNotIsNeitherAReadingNorASilence() {
        let partial = JSON.parse(#"""
            {"kind":"object","class":"Vibration","connected":true,"available":false,
             "silentReason":null,"silentText":null,"units":"m/s^2",
             "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                     {"axis":"y","label":"Y","value":null,"fraction":null,"severity":null},
                     {"axis":"z","label":"Z","value":null,"fraction":null,"severity":null}]}
            """#)

        XCTAssertNil(silentState(partial), "silentReason is set only when NO axis has a value, so a vehicle sending NaN in one axis of a VIBRATION message leaves this null")
        XCTAssertNil(vibrationReading(partial), "available is all three, so the same view produces no reading - the screen used to read reading!! after a silentState guard and threw on exactly this input")
        XCTAssertEqual(
            vibrationEmptyState(partial, vibrationReading(partial))?.title,
            PARTIAL_TITLE,
            "neither half covers it, so the screen has to; a guard that asks silentState alone sends this input to the bars it has no reading for"
        )
    }

    func testPartlyReportedWithoutAVehicleIsAConnectPromptNotAClaimAboutOne() {
        let latched = JSON.parse(#"""
            {"kind":"object","class":"Vibration","connected":false,"available":false,
             "silentReason":null,"silentText":null,"units":"m/s^2",
             "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                     {"axis":"y","label":"Y","value":null,"fraction":null,"severity":null},
                     {"axis":"z","label":"Z","value":null,"fraction":null,"severity":null}]}
            """#)

        XCTAssertEqual(
            vibrationEmptyState(latched, vibrationReading(latched))?.title,
            "No vehicle connected",
            "vibration facts latch after the vehicle goes, so silentReason stays null with nothing connected - saying what THIS VEHICLE is reporting would be a claim about one that is not there"
        )
    }

    func testTheScaleAndTheCaptionAreBuiltFromTheCoresOwnLevels() {
        XCTAssertEqual(scaleLabels(90.0, 30.0, 60.0), ["90", "60", "30", "0"])
        XCTAssertEqual(bandCaption(30.0, 60.0), "Under 30 healthy · 30-60 watch · over 60 unsafe")
    }

    func testABlankUnitLeavesNoEmptyBracketsInTheHeading() {
        XCTAssertEqual(vibrationHeading("m/s²"), "Vibration (m/s²)")
        XCTAssertEqual(vibrationHeading(""), "Vibration")
    }
}

final class SilentStateTests: XCTestCase {
    private func view(_ reason: String, _ text: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Vibration","connected":true,"available":false,"silentReason":\#(reason),"silentText":\#(text),"axes":[],"units":"m/s^2"}"#)
    }

    func testAReportingVehicleHasNoEmptyState() {
        XCTAssertNil(silentState(view("null", "null")))
    }

    func testTheTitleIsTheCoresSentenceAndTheBodyIsThisHeadsInstruction() throws {
        let gone = try XCTUnwrap(silentState(view(#""noVehicle""#, #""No vehicle is connected.""#)))

        XCTAssertEqual(gone.title, "No vehicle is connected.")
        XCTAssertEqual(gone.body, "Connect a vehicle from the Fly view to see its vibration levels.")
    }

    func testConnectedAndSilentGetsTheOtherInstructionChosenByTokenNotByWording() throws {
        let mute = try XCTUnwrap(silentState(view(#""notReported""#, #""This vehicle reports no vibration measurements.""#)))

        XCTAssertEqual(mute.title, "This vehicle reports no vibration measurements.")
        XCTAssertTrue(mute.body.hasPrefix("The autopilot has not sent a VIBRATION message"))
    }

    func testATokenThisHeadHasNeverSeenStillGetsABody() throws {
        let odd = try XCTUnwrap(silentState(view(#""somethingNew""#, #""Something new happened.""#)))

        XCTAssertEqual(odd.title, "Something new happened.")
        XCTAssertFalse(odd.body.isBlank)
    }

    func testNoViewAtAllIsTheDisconnectedCase() throws {
        XCTAssertEqual(try XCTUnwrap(silentState(nil)).title, "No vehicle connected")
    }

    private func reading(_ severities: [String?], clips: [Int] = [0, 0, 0]) -> VibrationReading {
        VibrationReading(
            units: "m/s/s",
            scaleMaximum: 90,
            warningLevel: 30,
            dangerLevel: 60,
            axes: severities.enumerated().map { index, severity in VibrationAxis(axis: ["X", "Y", "Z"][index], value: 1.0, fraction: 0.1, severity: severity) },
            clipCounts: clips
        )
    }

    func testTheVerdictNamesTheWorstAxesAndTheClipping() {
        XCTAssertEqual(vibrationVerdict(reading(["normal", "warning", "danger"], clips: [0, 3, 12])), "Vibration on Z is over the unsafe limit of 60. The accelerometers clipped 15 times; expect zero in flight.")
        XCTAssertEqual(vibrationVerdict(reading(["warning", "warning", "normal"])), "Vibration on X and Y is above 30; watch it. No clipping.")
        XCTAssertEqual(vibrationVerdict(reading(["normal", "normal", "normal"])), "Vibration is well under the limit. No clipping.")
    }
}
