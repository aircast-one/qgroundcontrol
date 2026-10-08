import XCTest
@testable import Aircast

final class BreachReturnTests: XCTestCase {
    func testTheBreachReturnPointAndItsAltitudeReadFromTheFencesView() {
        let read = breachReturn(JSON.parse(#"{"breachReturnPoint":{"latitude":47.4,"longitude":8.5},"breachReturnAltitude":{"value":98.4,"units":"ft","path":"plan.geoFenceController.breachReturnAltitude"}}"#))
        XCTAssertEqual(BreachReturn(point: TrackPoint(47.4, 8.5), altitude: 98.4, units: "ft", altitudePath: "plan.geoFenceController.breachReturnAltitude"), read)
        XCTAssertNil(breachReturn(JSON.parse(#"{"breachReturnPoint":null}"#)))
        XCTAssertEqual("98.4", breachAltitudeText(98.4))
    }
}
