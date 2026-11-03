import XCTest
@testable import Aircast

final class PayloadStatusTests: XCTestCase {
    private let at: Int64 = 1_000_000

    private func state(_ action: String, _ outcome: String, bay: Int? = nil) -> JSON {
        let bayField = bay.map { #","bay":\#($0)"# } ?? ""
        return JSON.parse(#"{"action":"\#(action)","outcome":"\#(outcome)","atMs":\#(at)\#(bayField)}"#)
    }

    func testSaysNothingUntilAPayloadCommandHasBeenSent() {
        XCTAssertNil(payloadStatus(nil, at))
        XCTAssertNil(payloadStatus(state("release", "lost"), at))
    }

    func testTellsThePilotWhatTheFlightControllerAnsweredAndHowLongAgo() {
        XCTAssertEqual(payloadStatus(state("release", "sent"), at + 500), PayloadStatus(text: "Drop sent, waiting for the aircraft", warning: false))
        XCTAssertEqual(payloadStatus(state("release", "confirmed"), at + 12_400), PayloadStatus(text: "Dropped 12 s ago", warning: false))
        XCTAssertEqual(payloadStatus(state("grab", "confirmed"), at + 65_000), PayloadStatus(text: "Grabbed 1 min ago", warning: false))
        XCTAssertEqual(payloadStatus(state("release", "refused"), at + 2_000), PayloadStatus(text: "Drop refused by the flight controller 2 s ago", warning: true))
    }

    func testADropTheAircraftNeverAnsweredIsAWarningNotAFailure() {
        XCTAssertEqual(
            payloadStatus(state("release", "unanswered"), at + 3_000),
            PayloadStatus(text: "Drop not confirmed 3 s ago. It may have dropped, check before dropping again.", warning: true)
        )
        XCTAssertEqual(
            payloadStatus(state("grab", "unanswered"), at + 3_000),
            PayloadStatus(text: "Grab not confirmed 3 s ago. Check the gripper before trying again.", warning: true)
        )
    }

    func testABasketNamesTheBayInEveryLine() {
        XCTAssertEqual(payloadStatus(state("release", "confirmed", bay: 2), at + 4_000), PayloadStatus(text: "Bay 2 dropped 4 s ago", warning: false))
        XCTAssertEqual(payloadStatus(state("release", "refused", bay: 3), at), PayloadStatus(text: "Drop bay 3 refused by the flight controller 0 s ago", warning: true))
    }

    func testTimeSinceIsCoarseAndNeverNegative() {
        XCTAssertEqual([agoText(at, at - 5_000), agoText(at, at + 59_999), agoText(at, at + 3_600_000)], ["0 s ago", "59 s ago", "1 h ago"])
    }
}
