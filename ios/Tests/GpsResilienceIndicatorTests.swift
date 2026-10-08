import XCTest
@testable import Aircast

final class GpsResilienceIndicatorTests: XCTestCase {
    func testMarksAndSectionsComeFromTheCore() throws {
        let shown = try XCTUnwrap(gpsResilience(JSON.parse(
            #"{"shown":true,"interference":{"shown":true,"colour":"error"},"authentication":{"shown":false,"colour":"neutral"},"#
                + #""sections":[{"title":"GPS Resilience Status","rows":[{"label":"GPS Jamming","text":"Not jammed"}]}]}"#
        )))
        XCTAssertTrue(shown.interference.shown)
        XCTAssertEqual(shown.interference.colour, "error")
        XCTAssertEqual(shown.sections[0].rows[0].0, "GPS Jamming")
        XCTAssertEqual(shown.sections[0].rows[0].1, "Not jammed")
        XCTAssertNil(gpsResilience(JSON.parse(#"{"shown":false}"#)))
    }
}
