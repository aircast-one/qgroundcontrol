import MapLibre
import XCTest
@testable import Aircast

final class GimbalWedgeTests: XCTestCase {
    func testEachGimbalsYawIsReadForTheMapWedge() {
        let read = gimbalAzimuths(JSON.parse(#"{"gimbals":[{"yaw":270.0,"active":true},{"yaw":10.0,"active":false}]}"#))
        XCTAssertEqual(read, [GimbalAzimuth(yaw: 270.0, active: true), GimbalAzimuth(yaw: 10.0, active: false)])
        XCTAssertEqual(gimbalAzimuths(nil), [])
    }

    func testAWedgeIsDrawnOnlyForAKnownYawOnAPlottedVehicle() {
        let gimbals = [GimbalAzimuth(yaw: 30, active: true), GimbalAzimuth(yaw: .nan, active: false), GimbalAzimuth(yaw: 200, active: false)]
        let drawn = gimbalFeatures(47, 8, gimbals).shapes.compactMap { $0 as? MLNPointFeature }
        XCTAssertEqual(drawn.map { $0.attribute(forKey: "yaw") as? Double }, [30, 200])
        XCTAssertEqual(drawn.map { $0.attribute(forKey: "opacity") as? Double }, [1.0, 0.4])
        XCTAssertTrue(gimbalFeatures(0, 0, gimbals).shapes.isEmpty)
        XCTAssertTrue(gimbalFeatures(.nan, 8, gimbals).shapes.isEmpty)
    }
}
