import XCTest
@testable import Aircast

final class VideoArTests: XCTestCase {
    func testTheCoresProjectionBecomesMarksAndLegsOnThePicture() {
        let reading = arReading(JSON.parse(#"{"available":true,"home":{"x":0.2,"up":-0.1,"label":"H","text":"120 m"},"waypoints":[{"x":0.5,"up":0.0,"label":"1"}],"route":[{"from":{"x":0.2,"up":-0.1},"to":{"x":0.5,"up":0.0}}]}"#))!
        XCTAssertEqual(reading.home, ArMark(at: ArSpot(x: 0.2, up: -0.1), label: "H", text: "120 m"))
        XCTAssertEqual(reading.waypoints.map(\.label), ["1"])
        XCTAssertEqual(reading.route.count, 1)
        XCTAssertNil(arReading(JSON.parse(#"{"available":false}"#)))
    }

    func testTheInterceptTargetDropRingAndLeadAreReadAsTheirOwnMarks() {
        let reading = arReading(JSON.parse(#"{"available":true,"aim":{"x":0.5,"up":0.1,"label":"Target","text":"100 m"},"impact":{"x":0.5,"up":0.0,"label":"Drop","text":"50 m"},"lead":{"x":0.6,"up":0.1},"leadLine":{"from":{"x":0.5,"up":0.1},"to":{"x":0.6,"up":0.1}},"waypoints":[],"route":[]}"#))!
        XCTAssertEqual(reading.aim, ArMark(at: ArSpot(x: 0.5, up: 0.1), label: "Target", text: "100 m"))
        XCTAssertEqual(reading.impact?.label, "Drop")
        XCTAssertEqual(reading.lead, ArSpot(x: 0.6, up: 0.1))
        XCTAssertEqual(reading.leadLine, ArLeg(from: ArSpot(x: 0.5, up: 0.1), to: ArSpot(x: 0.6, up: 0.1)))
        XCTAssertNil(arReading(JSON.parse(#"{"available":true}"#))?.aim)
    }

    func testASpotIsPlacedFromTheCentreLineInWidthsSoTheFramesAspectDoesNotDistortIt() {
        let centre = arPoint(ArSpot(x: 0.5, up: 0), CGSize(width: 1600, height: 900))
        XCTAssertEqual(centre.x, 800, accuracy: 1e-3)
        XCTAssertEqual(centre.y, 450, accuracy: 1e-3)
        XCTAssertEqual(arPoint(ArSpot(x: 0.5, up: 0.1), CGSize(width: 1600, height: 900)).y, 450 - 160, accuracy: 1e-3)
    }
}
