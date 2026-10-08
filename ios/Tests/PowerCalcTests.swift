import XCTest
@testable import Aircast

final class PowerCalcTests: XCTestCase {
    func testARowsCalculatorIsReadAndAsksTheCoreAboutItsOwnBatteryAndParameter() throws {
        let calculator = try XCTUnwrap(powerCalculator(JSON.parse(#"{"title":"Calculate Voltage Divider","measure":"voltage","batteryIndex":2,"param":"BAT2_V_DIV","button":"Calculate"}"#)))
        XCTAssertEqual(powerCalcPath(calculator), "view.powerCalc(voltage,2,BAT2_V_DIV)")
        XCTAssertNil(powerCalculator(nil))
        XCTAssertNil(powerCalculator(JSON.parse(#"{"title":"x"}"#)))
    }
}
