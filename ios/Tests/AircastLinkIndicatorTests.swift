import XCTest
@testable import Aircast

final class AircastLinkIndicatorTests: XCTestCase {
    func testTheLinkShowsOnlyOnceCellularTelemetryArrives() throws {
        XCTAssertNil(aircastLink(JSON.parse(#"{"shown":false}"#)))
        let link = try XCTUnwrap(aircastLink(JSON.parse(#"{"shown":true,"qualityText":"73%","signalText":"73 %","network":"LTE","modem":"connected","bitrateText":"2.5 Mbit/s","qualityHistory":[73,-1],"bitrateHistory":[2048,0]}"#)))
        XCTAssertEqual([link.cellText, link.network, link.modem, link.bitrateText], ["73%", "LTE", "connected", "2.5 Mbit/s"])
        XCTAssertEqual(link.qualityHistory, [73.0, -1.0])
        XCTAssertEqual(link.uplinkText, "")
        XCTAssertEqual(link.switches, [])
        XCTAssertTrue(link.cellular, "an older core that never says still has its modem rows")
        XCTAssertEqual(cellLabel(link), "73% 2.5 Mbit/s")
        XCTAssertEqual(aircastLinkRows(link).map(\.0), ["Signal:", "Network:", "Modem:", "Video bitrate:"])
    }

    func testTheCellNamesTheUplinkInUseAndTheSheetListsItsSwitchesNewestFirst() throws {
        let link = try XCTUnwrap(aircastLink(JSON.parse(#"{"shown":true,"quality":null,"qualityText":"--","signalText":"unknown","cellular":false,"bitrateText":"","uplink":"wifi","uplinkText":"Wi-Fi","cellText":"Wi-Fi","switches":[{"atMs":2000,"clock":"21:04:09","from":"LTE","to":"Wi-Fi"},{"atMs":1000,"clock":"21:03:41","from":"Starlink","to":"LTE"}]}"#)))
        XCTAssertEqual([link.cellText, link.uplinkText], ["Wi-Fi", "Wi-Fi"])
        XCTAssertFalse(link.cellular)
        XCTAssertEqual(cellLabel(link), "Wi-Fi", "no modem, so no bitrate after the uplink")
        XCTAssertEqual(aircastLinkRows(link).map { [$0.0, $0.1] }, [["Uplink:", "Wi-Fi"]])
        XCTAssertEqual(link.switches, [
            AircastLinkSwitch(clock: "21:04:09", from: "LTE", to: "Wi-Fi"),
            AircastLinkSwitch(clock: "21:03:41", from: "Starlink", to: "LTE"),
        ])
        XCTAssertEqual(aircastLink(JSON.parse(#"{"shown":true,"uplink":null,"uplinkText":"","cellText":"87%"}"#))?.uplinkText, "")
        XCTAssertEqual(aircastLink(JSON.parse(#"{"shown":true,"cellular":true,"uplink":"cellular","uplinkText":"LTE","cellText":"LTE 87%","bitrateText":"2.5 Mbit/s"}"#)).map(cellLabel), "LTE 87% 2.5 Mbit/s")
    }

    func testASparklineBreaksAtUnknownSamplesAndScalesToItsPeak() {
        let runs = sparkRuns([50.0, 100.0, -1.0, 0.0, 25.0, -1.0, 10.0], maximum: 0.0)
        XCTAssertEqual(runs.count, 2)
        XCTAssertEqual(runs[0].map { [$0.0, $0.1] }, [[0, 0.5], [Float(1) / 6, 0]])
        XCTAssertEqual(runs[1].map { [$0.0, $0.1] }, [[0.5, 1], [Float(4) / 6, 0.75]])
        XCTAssertTrue(sparkRuns([5.0], maximum: 100.0).isEmpty)
    }
}
