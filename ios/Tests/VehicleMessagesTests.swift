import XCTest
@testable import Aircast

final class VehicleMessagesTests: XCTestCase {
    private func message(_ index: Int, _ level: MessageSeverity, _ text: String) -> VehicleMessage {
        VehicleMessage(index: index, time: "12:00", severity: level.rawValue.lowercased(), level: level, text: text)
    }

    func testTheLogSummaryCountsErrorsAndWarningsApartFromTheRest() {
        let levelled = { (level: MessageSeverity) in VehicleMessage(index: 0, time: "", severity: "", level: level, text: "x") }
        XCTAssertEqual(severitySummary([levelled(.Error), levelled(.Warning), levelled(.Warning), levelled(.Normal)]), "1 error \u{00b7} 2 warnings \u{00b7} 1 message")
        XCTAssertEqual(severitySummary([]), "0 messages from the vehicle")
    }

    func testTheChipCarriesTheArmingBlockerAsAShortReason() {
        XCTAssertEqual(chipBlocker("PreArm: GPS 1: not healthy"), "GPS 1: not healthy")
        XCTAssertEqual(chipBlocker("No GPS lock. This vehicle needs a position fix before it will arm."), "No GPS lock")
        XCTAssertEqual(chipBlocker("Throttle too high"), "Throttle too high")
        let fly = FlyState(connected: true, armed: false, contactLost: false, state: "disarmed", stateText: "Not ready", staleNotice: "", mode: "Guided", rcSupported: false, rcSignalText: "", rcSignal: nil, rcOverride: nil, telemetry: nil)
        XCTAssertEqual(readinessSubtitle(fly, "PreArm: GPS 1: not healthy", 3), "Guided \u{00b7} GPS 1: not healthy +2")
        XCTAssertEqual(readinessSubtitle(fly, "Compass not calibrated", 1), "Guided \u{00b7} Compass not calibrated")
        XCTAssertNil(readinessSubtitle(fly, nil, 0))
    }

    func testTheBannerOnlyInterruptsForWarningsAndErrorsTheChipIsNotAlreadySaying() {
        let unread = [
            VehicleMessage(index: 0, time: "", severity: "", level: .Error, text: "PreArm: GPS 1: not healthy"),
            VehicleMessage(index: 0, time: "", severity: "", level: .Normal, text: "EKF3 IMU0 is using GPS"),
            VehicleMessage(index: 0, time: "", severity: "", level: .Warning, text: "Battery low"),
        ]
        XCTAssertEqual(bannerMessages(unread).map(\.text), ["Battery low"])
        XCTAssertTrue(bannerMessages(Array(unread.prefix(2))).isEmpty)
        XCTAssertEqual(bannerText(bannerMessages(unread)), "Battery low")
    }

    func testAMessageTimeDropsTheMillisecondsQgcStampsItWith() {
        XCTAssertEqual(messageTime("20:00:53.747"), "20:00:53")
        XCTAssertEqual(messageTime("yesterday"), "yesterday")
    }

    private let served = #"""
        {"kind":"object","class":"VehicleMessages","order":"oldestFirst","items":[
          {"index":0,"time":"1:2:3.4","component":null,"severity":"Error","level":"error","text":"Battery < 20% & \"low\""},
          {"index":1,"time":"1:2:4.0","component":190,"severity":"Notice","level":"warning","text":"heads up"},
          {"index":2,"time":"1:2:5.0","component":null,"severity":"","level":"normal","text":"plain"}]}
        """#

    func testMessagesComeFromTheCoreWithTheirLevelAndTime() {
        let messages = vehicleMessages(JSON.parse(served))
        XCTAssertEqual(messages.map(\.text), ["Battery < 20% & \"low\"", "heads up", "plain"])
        XCTAssertEqual(messages.map(\.level), [.Error, .Warning, .Normal])
        XCTAssertEqual(messages.map(\.time), ["1:2:3.4", "1:2:4.0", "1:2:5.0"])
        XCTAssertEqual(messages.map(\.index), [0, 1, 2])
    }

    func testTheLevelComesFromTheCoresTokenNotFromAWordThatGetsTranslated() {
        let german = JSON.parse(#"{"class":"VehicleMessages","order":"oldestFirst","items":[{"index":0,"time":"1:2:3.4","severity":"Fehler","level":"error","text":"kaputt"}]}"#)
        XCTAssertEqual(vehicleMessages(german).map(\.level), [.Error])
        XCTAssertEqual(vehicleMessages(german).map(\.severity), ["Fehler"])
    }

    func testAnUnknownLevelIsTreatedAsOrdinaryRatherThanDropped() {
        XCTAssertEqual(levelOf("something-new"), .Normal)
        XCTAssertEqual(levelOf(""), .Normal)
    }

    func testNothingToSayReadsAsAnEmptyList() {
        XCTAssertTrue(vehicleMessages(nil).isEmpty)
        XCTAssertTrue(vehicleMessages(JSON.parse(#"{"kind":"null"}"#)).isEmpty)
        XCTAssertTrue(vehicleMessages(JSON.parse(#"{"class":"VehicleMessages","items":[]}"#)).isEmpty)
    }

    func testAMessageWithNoTextIsNotShownAsABlankRow() {
        XCTAssertTrue(vehicleMessages(JSON.parse(#"{"class":"VehicleMessages","items":[{"index":0,"text":"","level":"error"}]}"#)).isEmpty)
    }

    func testAnArmingBlockerIsReadOnlyWhenTheCoreSetsOne() {
        XCTAssertEqual(armingBlocker(JSON.parse(#"{"armingBlocker":"Throttle too high"}"#)), "Throttle too high")
        XCTAssertNil(armingBlocker(JSON.parse(#"{"armingBlocker":null}"#)))
        XCTAssertNil(armingBlocker(JSON.parse(#"{"armingBlocker":""}"#)))
        XCTAssertNil(armingBlocker(nil))
    }

    func testAnErrorIsNamedInTheBannerNotBuriedInACount() {
        XCTAssertEqual(bannerText([message(0, .Normal, "Armed"), message(1, .Error, "EKF variance"), message(2, .Normal, "Mode changed")]), "EKF variance")
    }

    func testTheNewestErrorWinsAndTheCoreServesItsListsOldestFirst() {
        XCTAssertEqual(bannerText([message(0, .Error, "Compass variance"), message(1, .Error, "EKF variance")]), "EKF variance")
    }

    func testTheLogRendersNewestFirstWhichIsTheOrderQgcsOwnLogUses() {
        XCTAssertEqual([message(0, .Normal, "oldest"), message(1, .Normal, "newest")].reversed().map(\.text), ["newest", "oldest"])
    }

    func testAViewThatSaysNewestFirstIsTurnedRoundRatherThanTrustedToMatch() {
        let body = #"{"order":"newestFirst","items":[{"index":0,"time":"12:01","severity":"","level":"normal","text":"newest"},{"index":1,"time":"12:00","severity":"","level":"normal","text":"oldest"}]}"#
        XCTAssertEqual(vehicleMessages(JSON.parse(body)).map(\.text), ["oldest", "newest"])
    }

    func testTheOrderTheCoreActuallyServesIsTakenAsGiven() {
        let body = #"{"order":"\#(OLDEST_FIRST)","items":[{"index":0,"time":"12:00","severity":"","level":"normal","text":"oldest"},{"index":1,"time":"12:01","severity":"","level":"normal","text":"newest"}]}"#
        XCTAssertEqual(vehicleMessages(JSON.parse(body)).map(\.text), ["oldest", "newest"])
    }

    func testAWarningIsNamedWhenNothingWorseHasHappened() {
        XCTAssertEqual(bannerText([message(0, .Warning, "Low battery")]), "Low battery")
    }

    func testRoutineChatterDoesNotInterruptThePilot() {
        XCTAssertTrue(bannerMessages([message(0, .Normal, "Armed"), message(1, .Normal, "Disarmed")]).isEmpty)
    }

    func testNothingToSayIsNothingShown() {
        XCTAssertNil(bannerText([]))
    }

    func testTheDecoderReadsTheKeyTheCoreActuallyServes() {
        let recorded = JSON.parse(#"{"kind":"object","class":"VehicleMessages","count":1,"items":[{"index":0,"time":"1:2:3.4","component":null,"severity":"Error","level":"error","text":"real"}]}"#)
        XCTAssertEqual(vehicleMessages(recorded).map(\.text), ["real"])
        XCTAssertTrue(vehicleMessages(JSON.parse(#"{"class":"VehicleMessages","messages":[{"text":"x"}]}"#)).isEmpty, "a fixture invented by the head must not pass where the recorded one fails")
    }

    func testOpeningTheLogMarksWhatWasThereAsReadAsResetAllMessagesDoes() {
        let messages = [
            VehicleMessage(index: 0, time: "", severity: "error", level: .Error, text: "EKF variance"),
            VehicleMessage(index: 1, time: "", severity: "info", level: .Normal, text: "Armed"),
        ]
        XCTAssertEqual(unreadMessages(messages, 2).count, 2)
        XCTAssertEqual(unreadMessages(messages, 0), [], "an old error no longer turns the banner red")
        let newer = messages + [VehicleMessage(index: 2, time: "", severity: "warning", level: .Warning, text: "Low battery")]
        XCTAssertEqual(unreadMessages(newer, 1).map(\.text), ["Low battery"], "the core counts what arrived since resetAllMessages, newest last")
        XCTAssertEqual(unreadCount(JSON.parse(#"{"unread":1}"#)), 1)
        XCTAssertEqual(unreadCount(nil), 0)
        XCTAssertEqual(messageCountText(2), "2 messages from the vehicle")
    }

    private func warnings(_ checks: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"VehicleWarnings","warnings":[],"armingBlocker":"Compass not calibrated","armingChecks":\#(checks)}"#)
    }

    func testEveryReasonIsListedNotOnlyTheOneTheBannerHadRoomFor() throws {
        let listed = try XCTUnwrap(armingChecks(warnings(#"""
            [{"message":"Compass not calibrated","description":"Calibrate the compass.","severity":"error"},
             {"message":"GPS fix too poor","description":"Wait for more satellites.","severity":"error"},
             {"message":"Battery below 20%","description":"","severity":"warning"}]
            """#)))
        XCTAssertEqual(listed.map(\.message), ["Compass not calibrated", "GPS fix too poor", "Battery below 20%"], "armingBlocker is the FIRST error only - the core had sent all three the whole time")
        XCTAssertEqual(listed[2].severity, "warning")
    }

    func testNobodyAskedIsNotNothingIsWrong() {
        XCTAssertNil(armingChecks(warnings("null")), "warnings.rs serves null when the firmware has never sent a report")
        XCTAssertEqual(armingChecks(warnings("[]")), [])
    }

    func testAnEntryWithNoMessageIsNotAReason() {
        XCTAssertEqual(armingChecks(warnings(#"[{"message":"","description":"","severity":"error"}]"#)), [])
    }
}
