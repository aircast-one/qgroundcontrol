import XCTest
@testable import Aircast

final class PreviousCoordinateTests: XCTestCase {
    func testMoveToPreviousItemIsOfferedOnlyWhenTheCoreNamesAPreviousPosition() {
        let previous = previousCoordinate(JSON.parse(#"{"previousCoordinate":{"latitude":47.1,"longitude":8.1}}"#))
        XCTAssertEqual(47.1, previous?.0)
        XCTAssertEqual(8.1, previous?.1)
        XCTAssertNil(previousCoordinate(JSON.parse(#"{"previousCoordinate":null}"#)))
        XCTAssertNil(previousCoordinate(nil))
    }

    func testTheAltitudeHintIsTheCoresSentenceOrNothing() {
        XCTAssertEqual("Actual AMSL alt sent: 512.3 m", altitudeHint(JSON.parse(#"{"altitudeHint":"Actual AMSL alt sent: 512.3 m"}"#)))
        XCTAssertNil(altitudeHint(JSON.parse(#"{"altitudeHint":null}"#)))
    }

    func testALandingPatternOffersAltitudesRelativeToLaunch() {
        XCTAssertEqual(false, altitudesRelative(JSON.parse(#"{"landing":true,"altitudesAreRelative":false}"#)))
        XCTAssertNil(altitudesRelative(JSON.parse(#"{"landing":false,"altitudesAreRelative":null}"#)))
    }
}
