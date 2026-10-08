import XCTest
@testable import Aircast

final class ActuatorTestSectionTests: XCTestCase {
    private let motor = TestChannel(label: "Motor 1", function: 101, min: 0, max: 1, default: nil, isMotor: true)
    private let servo = TestChannel(label: "Servo 1", function: 201, min: -1, max: 1, default: 0, isMotor: false)

    func testAMotorSliderHasAStopZoneBelowMinThatSnapsLikeActuatorSlider() {
        XCTAssertEqual(motor.from, -0.15, accuracy: 1e-9)
        XCTAssertEqual(motor.rest, -0.15, accuracy: 1e-9)
        XCTAssertEqual(snapped(motor, -0.1), -0.15, accuracy: 1e-9)
        XCTAssertEqual(snapped(motor, -0.05), 0.0, accuracy: 1e-9)
        XCTAssertNil(sentValue(motor, -0.15))
        XCTAssertEqual(sentValue(motor, 0.4)!, 0.4, accuracy: 1e-9)
    }

    func testAServoSliderStartsAtItsDefaultAndHasNoStopZone() {
        XCTAssertEqual(servo.from, -1.0, accuracy: 1e-9)
        XCTAssertEqual(servo.rest, 0.0, accuracy: 1e-9)
        XCTAssertEqual(snapped(servo, -1.0), -1.0, accuracy: 1e-9)
    }
}

final class ActuatorActionsTests: XCTestCase {
    func testActionGroupsReadFromTheOutputsView() {
        let groups = actuatorActions(JSON.parse(#"{"actions":[{"label":"Set Spin Direction 1","type":4,"actions":[{"label":"Motor 1","function":101}]}]}"#))
        XCTAssertEqual(groups, [ActuatorActionGroup(label: "Set Spin Direction 1", type: 4, actions: [ActuatorActionChoice(label: "Motor 1", function: 101)])])
    }
}
