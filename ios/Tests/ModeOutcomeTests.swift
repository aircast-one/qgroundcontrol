import XCTest
@testable import Aircast

final class ModeOutcomeTests: XCTestCase {
    private let earlier = ModeAck(serial: 4, accepted: true, wording: "")

    func testAModeChangeSettlesIsRefusedByTheAckOrTimesOutLikeFlightModeIndicator() {
        XCTAssertEqual(modeOutcome("Loiter", earlier, earlier, true, 100), .Settled)
        XCTAssertEqual(modeOutcome("Loiter", earlier, earlier, false, 100), .Pending)
        XCTAssertEqual(modeOutcome("Loiter", earlier, ModeAck(serial: 5, accepted: false, wording: "denied"), false, 100), .Rejected("Loiter denied"))
        XCTAssertEqual(
            modeOutcome("Loiter", ModeAck(serial: 5, accepted: false, wording: "denied"), ModeAck(serial: 5, accepted: false, wording: "denied"), false, 100),
            .Pending,
            "an old refusal is not this one"
        )
        XCTAssertEqual(modeOutcome("Loiter", nil, nil, false, MODE_REPLY_MS), .Rejected("Loiter: no reply"))
    }

    func testTheCoresAckReadsIntoTheHead() {
        XCTAssertEqual(modeAck(JSON.parse(#"{"modeAck":{"serial":2,"accepted":false,"wording":"refused for now"}}"#)), ModeAck(serial: 2, accepted: false, wording: "refused for now"))
        XCTAssertNil(modeAck(JSON.parse(#"{"modeAck":null}"#)))
    }
}
