import XCTest
@testable import Aircast

private let TWO_DEAD = #"""
{"class": "Radio", "connected": true, "channelCount": 8, "liveChannels": 6,
 "summary": "8 channels reported, 6 carrying a signal.",
 "sticks": [{"key": "yaw", "title": "Yaw", "value": 1173, "valueText": "1173",
             "fraction": 0.173, "mapped": true, "reversed": true}],
 "channels": [{"index": 6, "label": "7", "value": 0, "valueText": "—", "fraction": 0.0, "live": false},
              {"index": 0, "label": "1", "value": 1607, "valueText": "1607", "fraction": 0.607, "live": true}]}
"""#

private let MID_CALIBRATION = #"""
{"class": "Radio", "connected": true, "channelCount": 8, "liveChannels": 8,
 "summary": "8 channels reported, 8 carrying a signal.",
 "calibrating": true, "statusText": "Move the Throttle stick all the way up and hold it there...",
 "nextText": "Next", "nextEnabled": true, "cancelEnabled": true, "skipEnabled": true,
 "sticks": [], "channels": []}
"""#

final class RadioViewTests: XCTestCase {
    func testACalibrationInProgressCarriesTheStepTextAndTheButtonsTheCoreEnables() throws {
        let cal = try XCTUnwrap(radioView(JSON.parse(MID_CALIBRATION))).calibration
        XCTAssertTrue(cal.running)
        XCTAssertEqual(cal.nextText, "Next")
        XCTAssertTrue(cal.cancelEnabled)
        XCTAssertTrue(cal.skipEnabled, "a step the operator cannot perform has to be skippable")
        XCTAssertTrue(cal.statusText.hasPrefix("Move the Throttle stick"))
    }

    func testAnIdleRadioOffersTheStartLabelAndNothingToCancel() throws {
        let idle = try XCTUnwrap(radioView(JSON.parse(TWO_DEAD))).calibration
        XCTAssertFalse(idle.running)
        XCTAssertFalse(idle.cancelEnabled, "nothing is running, so cancel would cancel nothing")
    }

    func testTooFewChannelsCarriesTheDialogQgcShowsInsteadOfStarting() throws {
        guard case .object(var fields) = JSON.parse(TWO_DEAD) else { return XCTFail("not an object") }
        fields["notReady"] = JSON.parse(#"{"title": "Not Ready", "message": "Please turn on RC transmitter."}"#)
        let short = try XCTUnwrap(radioView(.object(fields)))
        XCTAssertEqual(short.notReady?.first, "Not Ready")
        XCTAssertEqual(short.notReady?.second, "Please turn on RC transmitter.")
        XCTAssertNil(try XCTUnwrap(radioView(JSON.parse(TWO_DEAD))).notReady)
    }

    func testTheActionPathNamesTheControllerTheCoreReadsItsStateFrom() {
        XCTAssertEqual(radioCalAction("nextButtonClicked"), "radioCal.nextButtonClicked")
        XCTAssertEqual(radioCalAction("cancelButtonClicked"), "radioCal.cancelButtonClicked")
        XCTAssertEqual(RADIO_ENTER_ACTIONS.map(radioCalAction), ["radioCal.start", "radioCal.open"])
        XCTAssertEqual(RADIO_LEAVE_ACTIONS.map(radioCalAction), ["radioCal.cancelButtonClicked", "radioCal.close"])
    }

    func testAChannelCarryingNoSignalIsADashNeverAPwmOfZero() throws {
        let dead = try XCTUnwrap(try XCTUnwrap(radioView(JSON.parse(TWO_DEAD))).channels.first { !$0.live })
        XCTAssertEqual(dead.valueText, "—")
        XCTAssertNotEqual(dead.valueText, "0", "a reading of 0 would claim the receiver measured the stick at its floor")
    }

    func testTheSummaryCountsTheChannelsCarryingASignalNotTheChannelsReported() throws {
        let view = try XCTUnwrap(radioView(JSON.parse(TWO_DEAD)))
        XCTAssertEqual(view.channelCount, 8)
        XCTAssertEqual(view.summary, "8 channels reported, 6 carrying a signal.")
        XCTAssertTrue(view.summary.contains("6 carrying"), "the count the header used to show cannot say two are dead")
    }

    func testAReversedStickKeepsThatFlagWhichIsTheOnlyThingMarkingItOnTheRow() throws {
        let sticks = try XCTUnwrap(radioView(JSON.parse(TWO_DEAD))).sticks
        XCTAssertEqual(sticks.count, 1)
        XCTAssertTrue(sticks[0].reversed)
    }

    func testAnythingThatIsNotTheRadioViewIsRefusedRatherThanReadAsAnEmptyRadio() {
        XCTAssertNil(radioView(JSON.parse(#"{"class": "Track", "points": []}"#)))
        XCTAssertNil(radioView(nil))
    }
}

final class CalibrationStepTests: XCTestCase {
    func testTheLineTellingTheOperatorToClickIsDroppedBecauseTheButtonIsRightThere() {
        let served = "Lower the Throttle stick all the way down.\n\n" + "Reset all transmitter trims to center.\n\nClick Next to continue"
        XCTAssertEqual(calibrationStep(served), "Lower the Throttle stick all the way down.\n\nReset all transmitter trims to center.")
    }

    func testAStepThatDoesNotEndInAnInstructionToClickIsLeftAlone() {
        let served = "Move the Throttle stick all the way up and hold it there..."
        XCTAssertEqual(calibrationStep(served), served)
    }

    func testNothingServedIsNothingShown() {
        XCTAssertEqual(calibrationStep(""), "")
        XCTAssertEqual(calibrationStep("   \n  "), "")
    }

    func testTheZeroTrimsPromptComesFromTheCore() {
        let view = radioView(JSON.parse(#"{"class":"Radio","startPrompt":{"title":"Zero Trims","message":"Before calibrating"}}"#))
        XCTAssertEqual(view?.startPrompt?.first, "Zero Trims")
        XCTAssertEqual(view?.startPrompt?.second, "Before calibrating")
        XCTAssertNil(radioView(JSON.parse(#"{"class":"Radio","startPrompt":null}"#))?.startPrompt)
    }

    func testAStickReadingCarriesItsPulseUnitLikeThePenpotRadioRows() {
        XCTAssertEqual(stickReading(RadioStick(title: "Roll", valueText: "1605", fraction: 0.6, mapped: true, reversed: false)), "1605 \u{00b5}s")
        XCTAssertEqual(stickReading(RadioStick(title: "Yaw", valueText: "1595", fraction: 0.6, mapped: true, reversed: true)), "1595 \u{00b5}s \u{00b7} reversed")
        XCTAssertEqual(stickReading(RadioStick(title: "Yaw", valueText: "", fraction: 0, mapped: false, reversed: false)), "Not mapped")
        XCTAssertEqual(stickReading(RadioStick(title: "Throttle", valueText: "1100", fraction: 0.1, mapped: true, reversed: false, channel: 3)), "Ch 3 \u{00b7} 1100 \u{00b5}s")
    }
}
