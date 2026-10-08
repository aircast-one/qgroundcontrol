import XCTest
@testable import Aircast

private let NONE = "This vehicle is not reporting vibration"

final class AnalyzeNoteTests: XCTestCase {
    func testTheConsoleRowCarriesNoFirmwareNoteAsTheAnalyzeViewPageListHasNone() {
        XCTAssertNil(analyzeNote(.Console, true, nil))
    }

    func testAVibrationRowSaysWhenTheVehicleIsSendingNone() {
        XCTAssertEqual(analyzeNote(.Vibration, true, NONE), NONE)
        XCTAssertNil(analyzeNote(.Vibration, true, nil))
    }

    func testTheRowAndTheScreenAnswerPartlyReportedFromOneSource() {
        let view = { (available: Bool, reason: String) in JSON.parse(#"{"available":\#(available),"silentReason":\#(reason)}"#) }

        XCTAssertEqual(vibrationCaveat(view(false, #""notReported""#)), "This vehicle is not reporting vibration")
        XCTAssertEqual(
            vibrationCaveat(view(false, "null")),
            "This vehicle is reporting only some vibration axes",
            "available is all three axes, so saying 'not reporting' on a vehicle sending one of " +
                "them is a claim the view does not support - the row said it until the screen " +
                "beside it was fixed to distinguish them"
        )
        XCTAssertNil(vibrationCaveat(view(true, "null")))
        XCTAssertNil(vibrationCaveat(nil))
    }

    func testWithNoVehicleEveryRowStaysQuietRatherThanRepeatingTheSameSentenceFiveTimes() {
        AnalyzePage.allCases.forEach { page in
            XCTAssertNil(
                analyzeNote(page, false, NONE),
                "the screens themselves say to connect a vehicle, and five copies of it in a menu is noise"
            )
        }
    }

    func testTheRowsThisHeadCannotAnswerForCarryNothing() {
        [AnalyzePage.LogDownload, .Inspector, .GeoTag].forEach { page in
            XCTAssertNil(analyzeNote(page, true, NONE))
        }
    }

    func testTheListShowsUnreadMessagesAndTheVibrationVerdict() {
        XCTAssertTrue(analyzeStatus(.Messages, 3, nil) == ("3", .NeedsAttention))
        XCTAssertTrue(analyzeStatus(.Messages, 0, nil) == ("", .Neutral))
        XCTAssertTrue(analyzeStatus(.Vibration, 0, "danger") == ("Unsafe", .NeedsAttention))
        XCTAssertTrue(analyzeStatus(.Vibration, 0, "normal") == ("Healthy", .Done))
        XCTAssertTrue(analyzeStatus(.Console, 5, "danger") == ("", .Neutral))
    }

    func testTheMessagesRowReadsWhatTheVehicleHasSaidTheOthersWhatTheToolDoes() {
        let said = [
            VehicleMessage(index: 0, time: "", severity: "", level: .Warning, text: "Low battery"),
            VehicleMessage(index: 1, time: "", severity: "", level: .Normal, text: "Armed"),
        ]
        XCTAssertEqual(analyzeSubtitle(.Messages, said), "1 warning \u{00b7} 1 message")
        XCTAssertEqual(analyzeSubtitle(.Messages, []), AnalyzePage.Messages.description)
        XCTAssertEqual(analyzeSubtitle(.Console, said), AnalyzePage.Console.description)
    }

    func testTheVibrationRowReadsTheThreeAxesInTheirUnit() {
        let axis = { (name: String, value: Double?) in VibrationAxis(axis: name, value: value, fraction: 0, severity: nil) }
        let reading = VibrationReading(units: "m/s\u{00b2}", scaleMaximum: 60, warningLevel: 30, dangerLevel: 60, axes: [axis("X", 12.4), axis("Y", 8.6), axis("Z", 18.0)], clipCounts: [])
        let silent = VibrationReading(units: reading.units, scaleMaximum: reading.scaleMaximum, warningLevel: reading.warningLevel, dangerLevel: reading.dangerLevel, axes: [axis("X", nil)], clipCounts: reading.clipCounts)
        XCTAssertEqual(analyzeSubtitle(.Vibration, [], vibration: reading), "X 12 \u{00b7} Y 9 \u{00b7} Z 18 m/s\u{00b2}")
        XCTAssertEqual(analyzeSubtitle(.Vibration, [], vibration: silent), AnalyzePage.Vibration.description)
        XCTAssertEqual(analyzeSubtitle(.Vibration, [], vibration: nil), AnalyzePage.Vibration.description)
    }

    func testTheInspectorRowReadsTheTotalMessageRate() {
        let view = JSON.parse(#"{"available":true,"messages":[{"rateHz":5.0},{"rateHz":10.4},{"rateHz":0}]}"#)
        XCTAssertEqual(inspectorRateText(view), "15 messages/s")
        XCTAssertNil(inspectorRateText(JSON.parse(#"{"available":true,"messages":[]}"#)))
        XCTAssertEqual(analyzeSubtitle(.Inspector, [], vibration: nil, rate: nil), AnalyzePage.Inspector.description)
    }

    func testThePagesQGCCorePluginMarksRequiresVehicleSaySoWithNoVehicleLikeAnalyzeView() {
        XCTAssertEqual(
            Set(AnalyzePage.allCases.filter { analyzeGate($0, false) != nil }),
            [.LogDownload, .Vibration, .Inspector, .Console]
        )
        XCTAssertEqual(analyzeGate(.Console, false), "Requires a connected vehicle")
        AnalyzePage.allCases.forEach { XCTAssertNil(analyzeGate($0, true)) }
    }
}
