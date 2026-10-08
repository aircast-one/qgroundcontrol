import XCTest
@testable import Aircast

final class CommandRefusalTests: XCTestCase {
    func testACommandTheAircraftReachedSaysNothingAtAll() {
        XCTAssertNil(
            commandRefusal("Arm", true),
            "this is the direction that matters: every arm, disarm and mode change that works must leave no message, or the pilot learns to ignore the one that means something"
        )
        XCTAssertNil(commandRefusal("Loiter", true))
    }

    func testAnUnconfirmedCommandSaysItWasNotConfirmedNotThatItFailed() {
        XCTAssertEqual(
            commandRefusal("Arm", false),
            "Arm was not confirmed by the aircraft.",
            "attemptCommand gives up after COMMAND_SETTLE_MS and cannot tell a refused command from a slow one, so the sentence claims only what it knows"
        )
        XCTAssertEqual(commandRefusal("Disarm", false), "Disarm was not confirmed by the aircraft.")
    }

    func testAFlightModeNameReadsAsTheSubjectOfTheSentence() {
        XCTAssertEqual(
            commandRefusal("Altitude Hold", false),
            "Altitude Hold was not confirmed by the aircraft.",
            "the mode picker passes the mode's display name, so the sentence has to read with an arbitrary one in front of it - including the numeric names an unknown mode gets"
        )
        XCTAssertEqual(commandRefusal("Mode 65536", false), "Mode 65536 was not confirmed by the aircraft.")
    }

    func testALateConfirmationWithdrawsOnlyItsOwnSentence() {
        let late = "Arm was not confirmed by the aircraft."
        XCTAssertNil(withdrawn(late, late))
        XCTAssertEqual(
            withdrawn("Loiter was not confirmed by the aircraft.", late),
            "Loiter was not confirmed by the aircraft.",
            "a newer refusal shown since belongs to another command and stays"
        )
        XCTAssertNil(withdrawn(nil, late))
    }

    func testTheSettleWindowIsLongEnoughToBeAWaitAndShortEnoughToBeAnAnswer() {
        XCTAssertEqual(COMMAND_SETTLE_MS, 4000)
    }
}
