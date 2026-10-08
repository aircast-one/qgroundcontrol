import XCTest
@testable import Aircast

final class EspBridgeScreenTests: XCTestCase {
    func testTheBridgeReadsSettingsAndGroupedCounters() throws {
        XCTAssertNil(espBridge(JSON.parse(#"{"available":false}"#)))
        let bridge = try XCTUnwrap(espBridge(JSON.parse(#"""
            {"available":true,"modeIndex":null,"channel":6,"ssid":"PixRacer","password":"pixracer","ssidSta":null,"passwordSta":null,
            "baudRates":[57600,921600],"baudIndex":1,"hostPort":{"valueString":"14550"},
            "status":{"vehicle":{"received":1234567,"lost":0,"sent":null},"bridge":{},"qgc":{}}}
            """#)))
        XCTAssertNil(bridge.modeIndex)
        XCTAssertEqual(bridge.vehicle, LinkCounts(received: "1,234,567", lost: "0", sent: ""))
        XCTAssertEqual(bridge.hostPort, "14550")
        XCTAssertFalse(stationFieldsEnabled(bridge), "ESP8266Component.qml enables the STA fields only in station mode")
        var station = bridge
        station.modeIndex = 1
        XCTAssertTrue(stationFieldsEnabled(station))
    }

    func testRestartWaitsForTheBridgeLikeControllerBusy() {
        XCTAssertEqual(espBridge(JSON.parse(#"{"available":true}"#))?.busy, false)
        XCTAssertEqual(espBridge(JSON.parse(#"{"available":true,"busy":true}"#))?.busy, true)
    }

    func testTheQgcUdpPortIsTypedWithinTheValidatorRange() throws {
        let bridge = try XCTUnwrap(espBridge(JSON.parse(#"{"available":true,"hostPort":{"valueString":"14550","path":"vehicle.parameterManager.getParameter(240,WIFI_UDP_HPORT)"}}"#)))
        XCTAssertEqual(bridge.hostPortPath, "vehicle.parameterManager.getParameter(240,WIFI_UDP_HPORT)")
        XCTAssertEqual(["1023", "1024", "65535", "65536"].map(hostPortTyped), [nil, 1024, 65535, nil])
    }
}
