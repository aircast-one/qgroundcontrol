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

    func testAZeroStepOrAnEmptyRangeNeverCrashesTheSlider() {
        XCTAssertEqual(cameraSliderStep(0, 10, 0), 10.0 / 1001)
        XCTAssertNil(cameraSliderStep(5, 5, 0))
        XCTAssertNil(cameraSliderStep(5, 5, 1))
        XCTAssertNil(cameraSliderStep(10, 0, 1))
        XCTAssertNil(cameraSliderStep(0, 10, -1))
    }

    func testTheSliderStepsLikeComposeCoercingTheStepCount() {
        XCTAssertEqual(cameraSliderStep(-2, 2, 0.5), 0.5)
        XCTAssertNil(cameraSliderStep(0, 1, 1))
        XCTAssertEqual(cameraSliderStep(0, 1, 1e-300), 1.0 / 1001)
        XCTAssertEqual(cameraSliderStep(0, 10, 4), 5)
    }

    func testAHugeBoolValueIsOffRatherThanACrash() {
        let parameter = JSON.parse(#"{"name":"CAM_ON","isBool":true,"value":1e300}"#)
        XCTAssertEqual(cameraSettingControl(parameter), .Toggle(on: false))
        XCTAssertEqual(cameraSettingControl(JSON.parse(#"{"isBool":true,"value":1.5}"#)), .Toggle(on: true))
    }
}
