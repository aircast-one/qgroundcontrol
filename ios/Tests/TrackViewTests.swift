import XCTest
@testable import Aircast

private func served(_ points: Int, available: Bool = true, from: Int = 0, count: Int? = nil, generation: Int = 1, vehicleId: Int = 1) -> JSON {
    let listed = (0..<points).map { #"{"latitude": \#(47.0 + Double(from + $0) * 0.001), "longitude": 8.5}"# }.joined(separator: ",")
    return JSON.parse(
        #"{"class": "Track", "available": \#(available), "count": \#(count ?? points), "vehicleId": \#(vehicleId), "generation": \#(generation), "from": \#(from), "order": "oldestFirst", "points": [\#(listed)]}"#
    )
}

final class TrackViewTests: XCTestCase {
    func testTheTrailIsWhateverTheCoreRecordedAndNothingTheHeadDecided() throws {
        let reading = trackReading(served(3))
        XCTAssertEqual(reading.points.count, 3)
        XCTAssertEqual(try XCTUnwrap(reading.points.first).latitude, 47.0, accuracy: 1e-9)
    }

    func testOnePointIsAPositionNotALine() {
        XCTAssertFalse(trackDraws(trackReading(served(1))), "a single fix draws nothing")
        XCTAssertTrue(trackDraws(trackReading(served(2))))
    }

    func testAnUnavailableTrackDrawsNothingHoweverManyPointsCameWithIt() {
        XCTAssertFalse(trackDraws(trackReading(served(5, available: false))))
        XCTAssertFalse(trackDraws(trackReading(nil)))
    }

    func testAPointTheMapCannotPlotIsNotDrawnButKeepsItsPlaceInTheTrail() {
        let broken = JSON.parse(#"{"class": "Track", "available": true, "count": 2, "from": 0, "points": [{"latitude": 47.0, "longitude": 8.5}, {"latitude": null, "longitude": null}]}"#)
        XCTAssertEqual(trackReading(broken).points.count, 2)
        XCTAssertEqual(plottedTrack(trackReading(broken)).count, 1)
    }

    func testATailJoinsTheTrailAlreadyHeldAndReplacesItsMovedEndPoint() throws {
        let held = trackReading(served(300))
        let tail = trackReading(served(129, from: 299, count: 428))
        let merged = try XCTUnwrap(mergedTrack(held, tail))
        XCTAssertEqual(merged.points.count, 428)
        XCTAssertEqual(merged.from, 0)
        XCTAssertEqual(merged.points, trackReading(served(428)).points)
    }

    func testATailTheHeadCannotJoinAsksForTheWholeTrail() {
        let held = trackReading(served(300))
        XCTAssertNil(mergedTrack(held, trackReading(served(128, from: 400, count: 528))), "points between the held trail and the tail were missed")
        XCTAssertNil(mergedTrack(held, trackReading(served(128, from: 10, count: 138, generation: 2))), "a cleared trail is a new generation")
        XCTAssertNil(mergedTrack(held, trackReading(served(128, from: 10, count: 138, vehicleId: 2))), "another vehicle's trail")
        XCTAssertNil(mergedTrack(nil, trackReading(served(128, from: 10, count: 138))), "nothing held yet")
    }

    func testAShortTrailArrivesWholeInTheTail() {
        let tail = trackReading(served(5))
        XCTAssertEqual(mergedTrack(trackReading(served(300, generation: 7)), tail), tail)
    }
}
