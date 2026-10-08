import XCTest
@testable import Aircast

final class MotorGateTests: XCTestCase {
    private func frame(connected: Bool = true, armed: Bool = false, contactLost: String = "false") -> JSON {
        JSON.parse(#"{"kind":"object","class":"Frame","connected":\#(connected),"armed":\#(armed),"contactLost":\#(contactLost),"apmFirmware":false,"motorCount":4}"#)
    }

    func testAnArmedVehicleIsNotAVehicleToSpinAMotorOn() {
        XCTAssertFalse(canTest(motorGate(frame(armed: true)), true))
        XCTAssertEqual(motorRefusal(motorGate(frame(armed: true))), "The vehicle is armed. Disarm it before testing a motor.")
    }

    func testAVehicleThatStoppedAnsweringIsNotOneToSpinAMotorOn() {
        XCTAssertFalse(canTest(motorGate(frame(contactLost: "true")), true))
    }

    func testNobodyWatchingIsNeitherLostNorFineAndMustNotRefuse() {
        XCTAssertTrue(
            canTest(motorGate(frame(contactLost: "null")), true),
            "frame.rs withholds contactLost when the watch is off, because the raw flag stays false however long the vehicle has been silent. Refusing on unknown would ground a motor test that has nothing wrong with it"
        )
        XCTAssertNil(motorRefusal(motorGate(frame(contactLost: "null"))))
    }

    func testNoVehicleMeansNothingWillAnswer() {
        XCTAssertFalse(canTest(motorGate(frame(connected: false)), true))
        XCTAssertFalse(canTest(motorGate(nil), true))
    }

    func testTheSafetySwitchIsStillRequiredWithEverythingElseInOrder() {
        XCTAssertFalse(canTest(motorGate(frame()), false))
        XCTAssertTrue(canTest(motorGate(frame()), true))
    }

    func testStopIsNotATestAndDoesNotInheritATestsGate() {
        XCTAssertTrue(
            canStop(motorGate(frame(armed: true))),
            "the two conditions that are REASONS to stop - the vehicle arming while the motors turn, or the safety switch going off - both disabled the one control an operator reaches for. A control is enabled by what its own action requires, and sending zero throttle requires only a vehicle to send it to"
        )
        XCTAssertTrue(canStop(motorGate(frame(contactLost: "true"))))
        XCTAssertFalse(canStop(motorGate(frame(connected: false))))
    }

    func testArduPilotMotorsAreTestedByLetter() {
        XCTAssertEqual(motorLabel(3, true), "C")
        XCTAssertEqual(motorLabel(3, false), "3")
    }
}
