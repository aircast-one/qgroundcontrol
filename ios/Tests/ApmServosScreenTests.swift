import XCTest
@testable import Aircast

final class ApmServosScreenTests: XCTestCase {
    func testAnIdleOutputHasNoReadingAndTheSteppersMoveByOne() throws {
        let servos = apmServos(JSON.parse(#"""
            {"servos":[{"index":3,"pwm":null,"position":null,
            "min":{"kind":"control","control":"number","name":"SERVO3_MIN","path":"p.min","value":1100,"valueString":"1100"},
            "function":null,"trim":null,"max":null,"reversed":null}]}
            """#))
        XCTAssertEqual(servos.count, 1)
        let servo = try XCTUnwrap(servos.first)
        XCTAssertEqual(servo.index, 3)
        XCTAssertNil(servo.pwm)
        XCTAssertNil(servo.position)
        XCTAssertEqual(stepped(try XCTUnwrap(servo.min), -1), 1099.0)
        XCTAssertEqual(apmServos(nil), [])
    }
}
