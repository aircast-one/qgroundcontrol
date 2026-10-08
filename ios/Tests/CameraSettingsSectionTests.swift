import XCTest
@testable import Aircast

final class CameraSettingsSectionTests: XCTestCase {
    private let view = JSON.parse("""
        {"state":"ready","activeSettings":["CAM_EV","CAM_WBMODE","CAM_FLICKER","CAM_NAME"],"parameters":[
            {"name":"CAM_WBMODE","description":"White Balance","readOnly":false,"isBool":false,"value":1,"selected":1,"options":[{"label":"Auto","value":0},{"label":"Sunny","value":1}]},
            {"name":"CAM_EV","description":"Exposure Compensation","readOnly":false,"isBool":false,"value":0.5,"min":-2.0,"max":2.0,"step":0.5,"options":[]},
            {"name":"CAM_FLICKER","description":"Flicker","readOnly":true,"isBool":true,"value":1,"options":[]},
            {"name":"CAM_NAME","description":"Name","readOnly":false,"isBool":false,"value":"E90","options":[]},
            {"name":"CAM_HIDDEN","description":"Hidden","readOnly":false,"isBool":false,"value":0,"options":[]}]}
        """)

    func testOnlyTheActiveSettingsShowInTheCamerasOrderEachWithQgcsControl() {
        let shown = cameraSettings(view)
        XCTAssertEqual(shown.map(\.name), ["CAM_EV", "CAM_WBMODE", "CAM_FLICKER", "CAM_NAME"])
        XCTAssertEqual(shown[0].control, .Range(min: -2, max: 2, step: 0.5, value: 0.5))
        guard case .Choice(_, let selected) = shown[1].control else { return XCTFail("white balance is a choice") }
        XCTAssertEqual(selected, 1)
        XCTAssertEqual(shown[2].control, .Toggle(on: true))
        XCTAssertTrue(shown[2].readOnly)
        XCTAssertEqual(shown[3].control, .Entry(text: "E90"))
    }

    func testNothingShowsUntilTheDefinitionIsLoaded() {
        XCTAssertEqual(cameraSettings(JSON.parse(#"{"state":"fetching"}"#)), [])
        XCTAssertEqual(cameraSettings(nil), [])
    }
}
