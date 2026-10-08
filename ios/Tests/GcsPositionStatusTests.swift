import XCTest
@testable import Aircast

final class GcsPositionStatusTests: XCTestCase {
    func testRowsReadAsGcsPositionStatusDoesAndHideWithoutAValidPosition() throws {
        let position = JSON.parse(#"{"kind":"value","value":{"latitude":47.123456789,"longitude":8.5,"valid":true}}"#)
        let rows = try XCTUnwrap(gcsPositionRows(position, JSON.parse(#"{"value":2.345}"#)))
        XCTAssertEqual(rows.map { [$0.0, $0.1] }, [["Latitude", "47.1234568"], ["Longitude", "8.5000000"], ["HDOP", "2.3 m"]])
        XCTAssertEqual(try XCTUnwrap(gcsPositionRows(position, JSON.parse(#"{"value":0}"#))).last?.1, "N/A")
        XCTAssertNil(gcsPositionRows(JSON.parse(#"{"value":{"valid":false}}"#), nil))
        XCTAssertNil(gcsPositionRows(nil, nil))
    }
}
