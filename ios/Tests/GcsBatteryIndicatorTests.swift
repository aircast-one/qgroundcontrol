import UIKit
import XCTest
@testable import Aircast

final class GcsBatteryIndicatorTests: XCTestCase {
    func testThePhoneBatteryIsAskedOfTheCoreAsAPercentage() throws {
        XCTAssertEqual(phoneBattery(0.54, .unplugged), PhoneBattery(percent: 54, charging: false))
        XCTAssertEqual(phoneBattery(0.5, .charging), PhoneBattery(percent: 50, charging: true))
        XCTAssertEqual(phoneBattery(1, .full), PhoneBattery(percent: 100, charging: true))
        XCTAssertNil(phoneBattery(-1, .unknown))
        XCTAssertEqual(gcsBatteryPath(PhoneBattery(percent: 54, charging: false)), "view.gcsBattery(54,false)")
        let reading = try XCTUnwrap(gcsBatteryReading(JSON.parse(#"{"state":"low","levelText":"20%","stateText":"On battery","heading":"Ground Station","title":"Ground station battery"}"#)))
        XCTAssertEqual(reading.state, "low")
        XCTAssertEqual(reading.title, "Ground station battery")
    }
}
