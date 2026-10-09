import XCTest
@testable import Aircast

final class QgcTests: XCTestCase {
    private let fact = JSON.parse(#"{"name":"rawValue","shortDescription":"Map type","valueString":"Hybrid","value":2}"#)

    func testAFactReadAtAPathKeepsThatPath() {
        XCTAssertEqual("view.control(settings.map)", Qgc.factAt("view.control(settings.map)", fact).path)
        XCTAssertEqual("Map type", Qgc.factAt("view.control(settings.map)", fact).description)
    }

    func testAFactInAGroupIsNamedUnderTheGroup() {
        XCTAssertEqual("settings.map.rawValue", Qgc.fact("settings.map", fact).path)
        XCTAssertEqual("rawValue", Qgc.fact("", fact).path)
    }
}
