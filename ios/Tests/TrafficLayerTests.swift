import XCTest
@testable import Aircast

final class TrafficLayerTests: XCTestCase {
    func testContactsBecomeMarksLabelledAsTheMapItemLabelsThem() {
        let marks = trafficMarks(JSON.parse(
            #"{"units":{"altitude":"ft"},"contacts":[{"latitude":47.4,"longitude":8.5,"headingDegrees":90,"alert":true,"altitude":3280.8,"callsign":"UAL123"},{"latitude":null,"longitude":8.5,"alert":false,"altitude":100,"callsign":"X"}]}"#
        ))
        XCTAssertEqual(marks.count, 1)
        XCTAssertEqual(marks[0].label, "3281 ft\nUAL123")
        XCTAssertTrue(marks[0].alert)
        XCTAssertEqual(trafficLabel(nil, "ft", "UAL123"), "")
    }

    func testABlankCallsignLeavesJustTheAltitudeWithNoTrailingBreak() {
        XCTAssertEqual(trafficLabel(100, "ft", ""), "100 ft")
        XCTAssertEqual(trafficLabel(100, "", ""), "100")
        XCTAssertEqual(trafficLabel(100, "m", "AB12  "), "100 m\nAB12")
    }
}
