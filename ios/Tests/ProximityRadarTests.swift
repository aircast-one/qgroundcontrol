import XCTest
@testable import Aircast

final class ProximityRadarTests: XCTestCase {
    func testTheRadarShowsOnlyWhileDistanceTelemetryArrives() throws {
        XCTAssertNil(proximityRadar(JSON.parse(#"{"shown":false,"sectors":[]}"#)))
        let radar = try XCTUnwrap(proximityRadar(JSON.parse(#"{"shown":true,"rangeMeters":6,"sectors":[{"bearing":0,"meters":2.5,"text":"2.50"},{"bearing":45,"meters":null,"text":"–.––"}]}"#)))
        XCTAssertEqual(radar.range, 6.0, accuracy: 0.0)
        XCTAssertEqual(radar.sectors, [RadarSector(bearing: 0, meters: 2.5, text: "2.50"), RadarSector(bearing: 45, meters: nil, text: "–.––")])
    }
}
