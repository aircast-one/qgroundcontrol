import XCTest
@testable import Aircast

final class AircastLinkIndicatorTests: XCTestCase {
    func testTheLinkShowsOnlyOnceCellularTelemetryArrives() throws {
        XCTAssertNil(aircastLink(JSON.parse(#"{"shown":false}"#)))
        let link = try XCTUnwrap(aircastLink(JSON.parse(#"{"shown":true,"qualityText":"73%","signalText":"73 %","network":"LTE","modem":"connected","bitrateText":"2.5 Mbit/s","qualityHistory":[73,-1],"bitrateHistory":[2048,0]}"#)))
        XCTAssertEqual([link.qualityText, link.network, link.modem, link.bitrateText], ["73%", "LTE", "connected", "2.5 Mbit/s"])
        XCTAssertEqual(link.qualityHistory, [73.0, -1.0])
    }

    func testASparklineBreaksAtUnknownSamplesAndScalesToItsPeak() {
        let runs = sparkRuns([50.0, 100.0, -1.0, 0.0, 25.0, -1.0, 10.0], maximum: 0.0)
        XCTAssertEqual(runs.count, 2)
        XCTAssertEqual(runs[0].map { [$0.0, $0.1] }, [[0, 0.5], [Float(1) / 6, 0]])
        XCTAssertEqual(runs[1].map { [$0.0, $0.1] }, [[0.5, 1], [Float(4) / 6, 0.75]])
        XCTAssertTrue(sparkRuns([5.0], maximum: 100.0).isEmpty)
    }
}
