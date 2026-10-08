import XCTest
@testable import Aircast

final class ProximityRadarMapTests: XCTestCase {
    private let centre = TrackPoint(latitude: 47.4, longitude: 8.5)

    func testHiddenRadarReadsNothingAndSectorsWithoutADistanceAreDropped() {
        XCTAssertNil(radarReading(JSON.parse(#"{"shown":false}"#)))
        let reading = radarReading(JSON.parse(#"{"shown":true,"maxMeters":40,"sectors":[{"bearing":0,"meters":10},{"bearing":45,"meters":null}]}"#))
        XCTAssertEqual(reading, RadarReading(maxMeters: 40.0, sectors: [(0.0, 10.0)]))
        XCTAssertNil(radarReading(JSON.parse(#"{"shown":true,"maxMeters":null,"sectors":[]}"#))?.maxMeters)
    }

    func testLimitCircleClosesAndEachArcSpansItsSectorTurnedByHeading() throws {
        let lines = radarLines(centre, 90.0, RadarReading(maxMeters: 40.0, sectors: [(0.0, 10.0)]))
        XCTAssertEqual(lines.map(\.0), ["#FFFFFF", "#FF0000"])
        XCTAssertEqual(lines[0].1.first, lines[0].1.last)
        let arc = lines[1].1
        arc.forEach { XCTAssertEqual(metresBetween(centre, $0), 10.0, accuracy: 0.01) }
        XCTAssertEqual(try XCTUnwrap(arc.first).latitude, pointAt(centre, 10.0, 67.5).latitude, accuracy: 1e-9)
        XCTAssertEqual(try XCTUnwrap(arc.last).longitude, pointAt(centre, 10.0, 112.5).longitude, accuracy: 1e-9)
    }

    func testNoMaxDistanceDrawsOnlyTheArcs() {
        XCTAssertEqual(radarLines(centre, .nan, RadarReading(maxMeters: nil, sectors: [(180.0, 5.0)])).count, 1)
    }
}

final class FleetRadarTests: XCTestCase {
    private let reading = RadarReading(maxMeters: 40.0, sectors: [(0.0, 10.0)])

    func testEveryVehicleWithSensorsGetsItsOwnRadarTheActiveOneAtItsLivePosition() {
        let fleet = [
            VehicleChoice(id: 1, name: "A", state: "", link: "", contactLost: false, active: true, latitude: 1.0, longitude: 1.0, radar: reading),
            VehicleChoice(id: 2, name: "B", state: "", link: "", contactLost: false, active: false, latitude: 47.0, longitude: 8.0, heading: 90.0, radar: reading),
            VehicleChoice(id: 3, name: "C", state: "", link: "", contactLost: false, active: false, latitude: 48.0, longitude: 8.0),
        ]
        let placed = placedRadars(fleet, TrackPoint(latitude: 47.5, longitude: 8.5), 10.0)
        XCTAssertEqual(placed.map(\.at), [TrackPoint(latitude: 47.5, longitude: 8.5), TrackPoint(latitude: 47.0, longitude: 8.0)])
        XCTAssertEqual(placed.map(\.heading), [10.0, 90.0])
    }
}
