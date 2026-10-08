import XCTest
@testable import Aircast

final class GcsBatteryIndicatorTests: XCTestCase {
    func testThePhoneBatteryIsAskedOfTheCoreAsAPercentage() throws {
        XCTAssertEqual(phoneBattery(54, 100, 3), PhoneBattery(percent: 54, charging: false))
        XCTAssertEqual(phoneBattery(128, 256, 2), PhoneBattery(percent: 50, charging: true))
        XCTAssertNil(phoneBattery(-1, 100, 2))
        XCTAssertEqual(gcsBatteryPath(PhoneBattery(percent: 54, charging: false)), "view.gcsBattery(54,false)")
        let reading = try XCTUnwrap(gcsBatteryReading(JSON.parse(#"{"state":"low","levelText":"20%","stateText":"On battery","heading":"Ground Station","title":"Ground station battery"}"#)))
        XCTAssertEqual(reading.state, "low")
        XCTAssertEqual(reading.title, "Ground station battery")
    }
}
