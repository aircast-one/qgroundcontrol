import XCTest
@testable import Aircast

final class ModeOutcomeTests: XCTestCase {
    private let earlier = ModeAck(serial: 4, accepted: true)

    func testAModeChangeSettlesOrStopsWaitingOnARefusalOrSilenceWhichTheCoreWords() {
        XCTAssertEqual(modeOutcome(earlier, earlier, true, 100), .Settled)
        XCTAssertEqual(modeOutcome(earlier, earlier, false, 100), .Pending)
        XCTAssertEqual(modeOutcome(earlier, ModeAck(serial: 5, accepted: false), false, 100), .Unsettled)
        XCTAssertEqual(modeOutcome(ModeAck(serial: 5, accepted: false), ModeAck(serial: 5, accepted: false), false, 100), .Pending, "an old refusal is not this one")
        XCTAssertEqual(modeOutcome(nil, nil, false, MODE_REPLY_MS), .Unsettled)
    }

    func testTheCoresAckReadsIntoTheHead() {
        XCTAssertEqual(modeAck(JSON.parse(#"{"modeAck":{"serial":2,"accepted":false,"wording":"refused for now"}}"#)), ModeAck(serial: 2, accepted: false))
        XCTAssertNil(modeAck(JSON.parse(#"{"modeAck":null}"#)))
    }
}
