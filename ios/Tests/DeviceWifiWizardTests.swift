import XCTest
@testable import Aircast

final class DeviceWifiWizardTests: XCTestCase {
    func testAWifiJoinIsCheckedBeforeTheHotspotIsGivenUp() {
        XCTAssertEqual(wifiJoinProblem("  ", "", ""), DeviceRefusal(message: "Enter your network name first.", field: "ssid"))
        XCTAssertTrue((wifiJoinProblem("workshop", "", "workshop")?.message ?? "").contains("secured network"))
        XCTAssertNil(wifiJoinProblem("typed-by-hand", "", ""))
        XCTAssertTrue((wifiJoinProblem("workshop", "short", "")?.message ?? "").contains("8–63 characters"))
        XCTAssertEqual(wifiJoinProblem("workshop", String(repeating: "x", count: 64), "")?.field, "psk")
        XCTAssertNil(wifiJoinProblem("workshop", "supersecret", "workshop"))
    }

    func testAJoinThatLostTheLinkSaysHowToFindTheDeviceARefusedOneHidesThoseSteps() {
        let join = WifiJoinAttempt(attempt: 1, target: "workshop", status: "Switching…", instructions: true)
        XCTAssertTrue(joinOutcome(join, nil).status.contains("Connected to “workshop”"))
        let lost = joinOutcome(join, DEVICE_NO_ANSWER)
        XCTAssertTrue(lost.instructions)
        XCTAssertTrue(lost.status.contains("Lost contact"))
        XCTAssertEqual(
            joinOutcome(join, DeviceRefusal(message: "SSID is too long", field: "ssid")),
            WifiJoinAttempt(attempt: 1, target: "workshop", status: "✗ SSID is too long", instructions: false)
        )
    }

    func testReadsTheNetworksAScanFound() {
        XCTAssertEqual(
            parseWifiScan(JSON.parse(#"{"networks":[{"ssid":"workshop","signal":82,"secured":true}]}"#)),
            [WifiNetwork(ssid: "workshop", signal: 82, secured: true, active: false)]
        )
        XCTAssertNil(parseWifiScan(nil))
    }
}
