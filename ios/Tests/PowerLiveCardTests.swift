import XCTest
@testable import Aircast

final class PowerLiveCardTests: XCTestCase {
    func testTheTilesReadTheFirstPackInOrder() {
        let view = JSON.parse(#"{"available":true,"packs":[{"facts":[{"name":"mahConsumed","units":"mAh","valueString":"1800"},{"name":"voltage","units":"v","valueString":"11.10"},{"name":"current","units":"A","valueString":"32.00"}]}]}"#)
        XCTAssertEqual(
            powerTiles(view),
            [PowerTile(label: "VOLTAGE", value: "11.10", units: "V"), PowerTile(label: "CURRENT", value: "32.00", units: "A"), PowerTile(label: "USED", value: "1800", units: "mAh")]
        )
    }

    func testNoBatteryMeansNoCard() {
        XCTAssertEqual(powerTiles(JSON.parse(#"{"available":false}"#)), [])
        XCTAssertEqual(powerTiles(nil), [])
    }
}
