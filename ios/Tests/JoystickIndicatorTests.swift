import XCTest
@testable import Aircast

final class JoystickIndicatorTests: XCTestCase {
    func testTheBadgeReadsTheCoresIndicatorAndHidesWithoutAJoystick() {
        let badge = joystickBadge(JSON.parse(#"{"indicator":{"heading":"Xbox","enabledText":"No","warn":true,"typeText":"Gamepad","inputsText":"6 axes, 15 buttons"}}"#))
        XCTAssertEqual(badge, JoystickBadge(heading: "Xbox", enabledText: "No", warn: true, typeText: "Gamepad", inputsText: "6 axes, 15 buttons"))
        XCTAssertNil(joystickBadge(JSON.parse(#"{"indicator":null}"#)))
    }

    func testDetailRowsComeThroughWithTheirWarning() {
        let badge = joystickBadge(JSON.parse(#"{"indicator":{"heading":"Pad","details":[{"label":"Battery:","value":"15%","warn":true}]}}"#))
        XCTAssertEqual(badge?.details, [JoystickDetail(label: "Battery:", value: "15%", warn: true)])
        XCTAssertEqual(powerStateText(-1), "")
    }
}
