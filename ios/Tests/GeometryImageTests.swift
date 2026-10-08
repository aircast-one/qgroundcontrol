import XCTest
@testable import Aircast

final class GeometryImageTests: XCTestCase {
    private let quad = [
        GeometryMotor(index: 0, label: 1, x: 1, y: 1, counterClockwise: true),
        GeometryMotor(index: 1, label: 2, x: -1, y: -1, counterClockwise: true),
        GeometryMotor(index: 2, label: 3, x: 1, y: -1, counterClockwise: false),
        GeometryMotor(index: 3, label: 4, x: -1, y: 1, counterClockwise: false),
    ]

    func testTheQuadIsLaidOutNoseUpCentredAndInsideTheImage() {
        let layout = geometryLayout(quad, 320, 160)!
        let front = layout.motors.first { $0.motor.label == 1 }!
        let back = layout.motors.first { $0.motor.label == 2 }!
        XCTAssertTrue(front.center.y < back.center.y, "front is above back, as +x is drawn upwards")
        XCTAssertTrue(front.center.x > back.center.x, "+y is drawn to the right")
        XCTAssertEqual(layout.origin.x, (front.center.x + back.center.x) / 2, accuracy: 0.01)
        XCTAssertTrue(layout.motors.allSatisfy { $0.center.x - layout.rotorDiameter / 2 >= 0 && $0.center.y + layout.rotorDiameter / 2 <= 160 })
        XCTAssertNil(geometryLayout(Array(quad.prefix(1)), 320, 160), "one motor draws nothing")
    }

    func testACoaxialPairDrawsTheLowerMotorOffsetAndATapPicksOnlyAHighlightedMotor() {
        let coax = quad + [GeometryMotor(index: 4, label: 5, x: 1, y: 1, counterClockwise: false)]
        let layout = geometryLayout(coax, 320, 160)!
        let lower = layout.motors.first { $0.motor.label == 5 }!
        XCTAssertTrue(lower.coax)
        XCTAssertTrue(layout.extraYMargin > 0)
        let target = layout.motors.first { $0.motor.label == 3 }!
        XCTAssertEqual(2, motorAt(layout, target.textCenter, [2]))
        XCTAssertNil(motorAt(layout, target.textCenter, []))
        XCTAssertNil(motorAt(layout, CGPoint(x: -50, y: -50), [0, 1, 2, 3]))
    }
}
