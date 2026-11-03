import Foundation
import XCTest
@testable import Aircast

final class DeviceViewFixturesTests: XCTestCase {
    private func served(_ page: String) throws -> JSON {
        let fixtures = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("android/app/src/test/resources/aircastd")
        let json = JSON.parse(try String(contentsOf: fixtures.appendingPathComponent("\(page).json"), encoding: .utf8))
        XCTAssertNotNil(json.object, "\(page).json did not parse")
        return json
    }

    func testEveryDevicePageReadsWhatAircastdServes() throws {
        XCTAssertEqual(parseDeviceStatus(try served("aircraft"))?.headline, "Ready to fly")
        XCTAssertFalse(try XCTUnwrap(parseTelemetry(try served("telemetry"))).endpoints.isEmpty)
        XCTAssertFalse(try XCTUnwrap(parseConnect(try served("connect"))).choices.isEmpty)
        XCTAssertEqual(parseFlights(try served("flights"))?.flights, [])
        XCTAssertFalse(try XCTUnwrap(parseDeviceSettings(try served("device"))).stats.isEmpty)
        XCTAssertNotNil(parseEngineering(try served("engineering")))
        XCTAssertNotNil(parseVideo(try served("video")))
        XCTAssertEqual(parseDetection(try served("detection"))?.cameras.first?.name, "cam2")
        XCTAssertFalse(try XCTUnwrap(parseRecordings(try served("recordings"))).limit.options.isEmpty)
        XCTAssertEqual(parseDeviceSetup(try served("setup"))?.steps.first?.key, "network")
    }

    // MARK: - Connection (DeviceNetwork)

    func testTheConnectionPageReadsWhatAircastdServes() throws {
        XCTAssertEqual(parseNetwork(try served("network"))?.uplink.title, "Wi-Fi (wlan0)")
    }
}
