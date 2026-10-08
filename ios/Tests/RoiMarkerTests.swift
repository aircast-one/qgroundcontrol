import XCTest
@testable import Aircast

final class RoiMarkerTests: XCTestCase {
    func testTheMarkerShowsOnlyWhileTheVehicleReportsAnActiveRoi() {
        XCTAssertEqual(roiPoint(JSON.parse(#"{"roiActive":true,"roi":{"latitude":47.4,"longitude":8.5}}"#)), TrackPoint(latitude: 47.4, longitude: 8.5))
        XCTAssertNil(roiPoint(JSON.parse(#"{"roiActive":false,"roi":null}"#)))
        XCTAssertNil(roiPoint(JSON.parse(#"{"roi":{"latitude":0.0,"longitude":0.0}}"#)))
        XCTAssertNil(roiPoint(nil))
    }
}
