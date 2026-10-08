import XCTest
@testable import Aircast

final class MapTypesTests: XCTestCase {
    func testThePlanMapMenuListsTheProvidersTypesAndMarksTheCurrentOne() {
        let read = mapTypes(JSON.parse(#"{"current":"Satellite","types":["Street Map","Satellite"],"path":"settings.flightMapSettings.mapType.rawValue"}"#))
        XCTAssertEqual(MapTypes(current: "Satellite", types: ["Street Map", "Satellite"], path: "settings.flightMapSettings.mapType.rawValue"), read)
        XCTAssertNil(mapTypes(JSON.parse(#"{"kind":"null"}"#)))
    }
}
