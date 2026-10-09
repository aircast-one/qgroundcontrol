import XCTest
@testable import Aircast

final class BatteryReturnTests: XCTestCase {
    private func battery(_ state: Int, _ power: String, _ current: String) -> JSON? {
        JSON.parse(#"{"available":true,"packs":[{"chargeState":\#(state),"facts":[{"name":"instantPower","value":\#(power)},{"name":"current","value":\#(current)}]}]}"#)
    }

    func testACriticalOrWorsePackOffersReturn() {
        XCTAssertTrue(batteryReturnOffered(battery(3, "120", "10")))
        XCTAssertTrue(batteryReturnOffered(battery(6, "120", "10")))
        XCTAssertFalse(batteryReturnOffered(battery(2, "120", "10")))
        XCTAssertFalse(batteryReturnOffered(nil))
    }

    func testTotalDrawSumsWattsElseAmps() {
        XCTAssertEqual(totalDraw(battery(1, "119.6", "10")), "120W")
        XCTAssertEqual(totalDraw(battery(1, "null", "10")), "10.0A")
        XCTAssertNil(totalDraw(battery(1, "null", "null")))
    }
}
