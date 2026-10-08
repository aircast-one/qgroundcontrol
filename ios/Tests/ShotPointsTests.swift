import MapLibre
import XCTest
@testable import Aircast

final class ShotPointsTests: XCTestCase {
    private func video(_ points: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Video","available":true,"decoding":true,"shotPoints":\#(points)}"#)
    }

    func testEveryPhotoTheVehicleReportedGetsAPlaceOnTheMap() {
        let points = shotPoints(video(#"[{"latitude":47.397,"longitude":8.545},{"latitude":47.398,"longitude":8.546}]"#))
        XCTAssertEqual(2, points.count)
        XCTAssertEqual(47.397, points[0].latitude, accuracy: 1e-9)
        XCTAssertEqual(8.546, points[1].longitude, accuracy: 1e-9)
    }

    func testAVehicleThatHasTakenNoPhotosDrawsNothing() {
        XCTAssertEqual([], shotPoints(video("[]")))
        XCTAssertEqual([], shotPoints(JSON.parse(#"{"kind":"object"}"#)))
        XCTAssertEqual([], shotPoints(nil))
    }

    func testAPointWithoutAUsableCoordinateIsDroppedRatherThanPlottedAtZero() {
        let points = shotPoints(video(#"[{"latitude":47.397,"longitude":8.545},{"longitude":8.546},{"latitude":null,"longitude":null}]"#))
        XCTAssertEqual(1, points.count)
        XCTAssertTrue((points.first?.latitude ?? 0) > 47.0)
    }

    func testTheFeaturesHandedToTheMapCarryLongitudeFirstAsGeoJsonWantsIt() throws {
        let collection = shotFeatures([TrackPoint(47.397, 8.545)])
        let json = try XCTUnwrap(String(data: collection.geoJSONData(usingEncoding: String.Encoding.utf8.rawValue), encoding: .utf8))
        XCTAssertTrue(json.contains("8.545"), "GeoJSON is [longitude, latitude] and swapping them lands the shot in Somalia")
        XCTAssertEqual(1, collection.shapes.count)
        let longitude = try XCTUnwrap(json.range(of: "8.545"))
        let latitude = try XCTUnwrap(json.range(of: "47.397"))
        XCTAssertTrue(longitude.lowerBound < latitude.lowerBound)
    }
}
