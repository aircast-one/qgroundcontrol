import XCTest
@testable import Aircast

final class GimbalIndicatorTests: XCTestCase {
    func testTheCellReadsStatusPitchAndYawAsTheToolbarDoes() {
        let state = gimbalIndicator(JSON.parse(#"{"shown":true,"statusText":"Yaw follow","pitchText":"P: -12.3","yawText":"Y: 5.0","yawLockLabel":"Yaw Lock","retractOffered":true,"controlOffered":false,"controlLabel":"Acquire Control","gimbals":[{"name":"Gimbal 1","managerCompid":1,"deviceId":154,"active":true}]}"#))!
        XCTAssertEqual(gimbalCellText(state), "Yaw follow · P: -12.3 · Y: 5.0")
        var two = state
        two.gimbals.append(GimbalChoice(name: "1-155", managerCompid: 1, deviceId: 155, active: false))
        XCTAssertEqual(gimbalCellText(two), "Gimbal 1 · Yaw follow · P: -12.3 · Y: 5.0", "with several gimbals the toolbar names the active one")
        XCTAssertTrue(state.retractOffered)
        XCTAssertEqual(state.gimbals[0].deviceId, 154)
        XCTAssertNil(gimbalIndicator(JSON.parse(#"{"shown":false}"#)))
    }

    func testATapMapsToQgcsCookedScreenFractions() {
        let centre = screenFraction(50, 50, 100, 100)
        XCTAssertEqual(centre.0, 0)
        XCTAssertEqual(centre.1, 0)
        let corner = screenFraction(100, 0, 100, 100)
        XCTAssertEqual(corner.0, 1)
        XCTAssertEqual(corner.1, 1)
        XCTAssertEqual(screenFraction(0, 100, 100, 100).1, -1)
        XCTAssertEqual(onScreenGimbal(JSON.parse(#"{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":true}}"#)), OnScreenGimbal(enabled: true, clickAndDrag: true))
        XCTAssertEqual(onScreenGimbal(JSON.parse(#"{"shown":true,"onScreen":{"enabled":false,"clickAndDrag":true}}"#)), OnScreenGimbal(enabled: false, clickAndDrag: true), "with on-screen control off the video still aims by drag, as CameraAimArea")
    }
}

final class GimbalSettingsTests: XCTestCase {
    private func fact(_ name: String, _ value: Any) -> Fact {
        Fact(
            path: "settings.gimbalControllerSettings.\(name)", name: name, description: name, units: "",
            valueString: "\(value)", value: JSON(value), enumStrings: [], enumIndex: -1,
            isBool: value is Bool, isString: false, readOnly: false
        )
    }

    private var flyView: [SettingsSectionRows] {
        [
            SettingsSectionRows(title: "Fly View", group: "flyViewSettings", note: "", blocks: [SettingsBlock(title: "", facts: [fact("showGimbalOnScreenControl", true)])]),
            SettingsSectionRows(title: "Gimbal Controller", group: "gimbalControllerSettings", note: "", blocks: [
                SettingsBlock(title: "On-Screen Control", facts: [fact("enableOnScreenControl", true), fact("clickAndDrag", false)]),
                SettingsBlock(title: "Zoom speed", facts: [fact("zoomMaxSpeed", 100), fact("zoomMinSpeed", 1)]),
                SettingsBlock(title: "", facts: [fact("joystickButtonsSpeed", 30), fact("showAzimuthIndicatorOnMap", false)]),
            ]),
        ]
    }

    func testTheSheetShowsTheGimbalSectionsHeadedBlocksLikeGimbalIndicatorsExpandedPage() {
        XCTAssertEqual(gimbalSettingsBlocks(flyView, true).map(\.title), ["On-Screen Control", "Zoom speed", ""])
        XCTAssertEqual(gimbalSettingsBlocks(flyView, true).first?.facts.map(\.name), ["enableOnScreenControl", "clickAndDrag"])
    }

    func testJoystickButtonsSpeedIsReadOnlyWithoutAJoystickEnabledForTheVehicle() {
        let speed = { (buttons: Bool) in gimbalSettingsBlocks(self.flyView, buttons).flatMap(\.facts).first { $0.name == "joystickButtonsSpeed" }! }
        XCTAssertFalse(speed(false).enabled)
        XCTAssertTrue(speed(true).enabled)
        XCTAssertFalse(joystickButtonsAvailable(JSON.parse(#"{"active":null,"vehicle":true,"enabled":true}"#)))
        XCTAssertTrue(joystickButtonsAvailable(JSON.parse(#"{"active":"Pad","vehicle":true,"enabled":true}"#)))
    }
}

final class GimbalTakeControlTests: XCTestCase {
    func testAnotherHolderOfTheGimbalAsksToTakeControlInsteadOfFailingQuietly() {
        gimbalAsksForControl.value = false
        XCTAssertNil(gimbalRefusal(JSON.parse(#"{"ok":false,"refusal":"othersHaveControl","reason":"Command not sent. Another user has control of the gimbal."}"#)))
        XCTAssertTrue(gimbalAsksForControl.value)
        XCTAssertEqual(gimbalRefusal(JSON.parse(#"{"ok":false,"refusal":"notReady","reason":"The gimbal is not ready yet."}"#)), "The gimbal is not ready yet.")
        gimbalAsksForControl.value = false
    }

    func testAJoystickRefusalRaisedInTheCoreAsksOnceTheSerialMovesNeverOnTheFirstRead() {
        XCTAssertFalse(serialAsks(nil, 3))
        XCTAssertFalse(serialAsks(3, 3))
        XCTAssertTrue(serialAsks(3, 4))
    }

    func testADragAimsByItsMoveOverHalfTheViewWidthAsCameraAimAreasSensitivity() {
        XCTAssertEqual(aimFraction(100, 400), 0.5)
        XCTAssertEqual(aimFraction(-50, 400), -0.25)
    }

    func testTheFirstDragEventOnlyMarksTheStartSoTheTouchSlopNeverJerksTheGimbal() {
        XCTAssertNil(aimDelta(nil, CGSize(width: 10, height: 0)))
        XCTAssertEqual(aimDelta(CGSize(width: 10, height: 0), CGSize(width: 14, height: -3)), CGSize(width: 4, height: -3))
    }
}
