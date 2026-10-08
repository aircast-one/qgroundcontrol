import XCTest
@testable import Aircast

final class JoystickScreenTests: XCTestCase {
    func testThePageReadsDevicesSettingsAndAxes() throws {
        XCTAssertNil(joystickPage(JSON.parse(#"{"available":false}"#)))
        let page = try XCTUnwrap(joystickPage(JSON.parse(#"""
            {"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":false,"calibrated":false,
            "settings":[{"name":"useDeadband","type":"bool","label":"Use deadband","units":"","value":true}],
            "state":{"axes":[{"index":0,"raw":1200,"function":"roll"}]}}
            """#)))
        XCTAssertEqual(page.active, "Pad")
        XCTAssertEqual(page.settings.count, 1)
        XCTAssertEqual(page.settings.first?.value, .bool(true))
        XCTAssertEqual(page.axes, [JoystickAxis(index: 0, raw: 1200, function: "roll")])
    }

    func testGamepadValuesScaleToTheCoreRangeAndTheDpadBecomesHatBits() {
        XCTAssertEqual(scaled(1), 32767)
        XCTAssertEqual(scaled(-2), -32767)
        XCTAssertEqual(hatBits(-1, -1), 0x01 | 0x08)
        XCTAssertEqual(hatBits(1, 1), 0x04 | 0x02)
        XCTAssertEqual(hatBits(0.2, -0.2), 0)
    }

    func testTheCalibrationPanelReadsTheCoreWizard() {
        let idle = joystickCalibration(nil)
        XCTAssertEqual(idle.nextText, "Calibrate")
        let step = joystickCalibration(JSON.parse(#"{"calibrating":true,"statusText":"Move the Throttle stick","nextText":"Next","nextEnabled":false,"cancelEnabled":true,"oneSidedVisible":true}"#))
        XCTAssertEqual(step, JoystickCalibration(calibrating: true, statusText: "Move the Throttle stick", nextText: "Next", nextEnabled: false, cancelEnabled: true, oneSidedVisible: true))
    }

    func testButtonsShowTheirActionAndWhetherTheyAreHeld() throws {
        let page = try XCTUnwrap(joystickPage(JSON.parse(#"""
            {"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":true,"calibrated":true,"settings":[],
            "state":{"axes":[],"buttons":[{"index":0,"action":null,"repeat":false,"event":"none"},{"index":1,"action":"Step Zoom In","repeat":true,"event":"repeat"}]},
            "assignableActions":[{"action":"No Action","canRepeat":false},{"action":"Step Zoom In","canRepeat":true}]}
            """#)))
        XCTAssertEqual(page.buttons, [JoystickButton(index: 0, action: "No Action", repeat: false, pressed: false), JoystickButton(index: 1, action: "Step Zoom In", repeat: true, pressed: true)])
        XCTAssertEqual(page.actions[1], AssignableAction(action: "Step Zoom In", canRepeat: true))
    }

    func testCalibrationReadsTheStickDiagramPositions() {
        let cal = joystickCalibration(JSON.parse(#"{"calibrating":true,"stickPositions":[0,1,0,0],"singleStickDisplay":false}"#))
        XCTAssertEqual(cal.stickPositions, [0, 1, 0, 0])
        XCTAssertEqual(joystickCalibration(nil).stickPositions, [0, 0, 0, 0])
    }

    func testASettingTheCoreHidesIsNotListed() throws {
        let page = try XCTUnwrap(joystickPage(JSON.parse(#"{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"negativeThrust","type":"bool","label":"Negative thrust","units":"","value":false,"visible":false},{"name":"exponentialPct","type":"double","label":"Exponential","units":"%","value":0}]}"#)))
        XCTAssertEqual(page.settings.map(\.name), ["exponentialPct"])
        let ranged = try XCTUnwrap(joystickPage(JSON.parse(#"{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"exponentialPct","type":"double","label":"Stick Exponential","units":"%","value":0,"slider":{"from":0.0,"to":50.0,"decimals":1}}]}"#)))
        XCTAssertEqual(ranged.settings.first?.slider, FactSlider(from: 0, to: 50, decimals: 1, hint: ""))
    }
}
