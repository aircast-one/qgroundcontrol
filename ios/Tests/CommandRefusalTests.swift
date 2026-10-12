import XCTest
@testable import Aircast

final class CommandRefusalTests: XCTestCase {
    func testTheAircraftsReplyToAModeChangeIsToldOnceByTheCoreNotRepeatedInTheBanner() {
        XCTAssertTrue(isModeReply("No such mode 26"))
        XCTAssertTrue(isModeReply("Mode change to AUTO failed: requires position"))
        XCTAssertFalse(isModeReply("Arm: Mode not armable"))
        let reply = VehicleMessage(index: 0, time: "12:00:00", severity: "Warning", level: .Warning, text: "No such mode 26")
        let fence = VehicleMessage(index: 1, time: "12:00:00", severity: "Warning", level: .Warning, text: "Fence breached")
        XCTAssertEqual(bannerMessages([reply, fence]), [fence])
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
