import XCTest
@testable import Aircast

final class CommandRefusalTests: XCTestCase {
    func testALateConfirmationWithdrawsOnlyItsOwnSentence() {
        let late = "Loiter was not confirmed by the aircraft."
        XCTAssertNil(withdrawn(late, late))
        XCTAssertEqual(
            withdrawn("Altitude Hold was not confirmed by the aircraft.", late),
            "Altitude Hold was not confirmed by the aircraft.",
            "a newer refusal shown since belongs to another command and stays"
        )
        XCTAssertNil(withdrawn(nil, late))
    }

    func testTheAircraftsReplyToAnArmAttemptIsToldOnceByTheCoreNotRepeatedInTheBanner() {
        XCTAssertTrue(isArmReply("Arm: Mode not armable"))
        XCTAssertTrue(isArmReply("Arming denied: Flight mode not allowed"))
        XCTAssertFalse(isArmReply("EKF3 IMU0 is using GPS"))
        let reply = VehicleMessage(index: 0, time: "12:00:00", severity: "Critical", level: .Error, text: "Arm: Mode not armable")
        let crash = VehicleMessage(index: 1, time: "12:00:00", severity: "Critical", level: .Error, text: "Crash: Disarming")
        XCTAssertEqual(bannerMessages([reply, crash]), [crash])
    }

    func testSentIsABriefAcknowledgementGoneBeforeARefusalCouldSitUnderIt() {
        XCTAssertEqual(SENT_SHOWN_MS, 3_000)
    }
}
