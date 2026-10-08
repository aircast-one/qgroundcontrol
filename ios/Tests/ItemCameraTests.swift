import XCTest
@testable import Aircast

final class ItemCameraTests: XCTestCase {
    private func view(
        available: Bool = true,
        commandsGimbal: Bool = true,
        action: String = #"{"value":1,"text":"Take photo","units":""}"#,
        pitch: String = #"{"value":-45.0,"text":"-45.0","units":"deg"}"#,
        yaw: String = #"{"value":0.0,"text":"0.0","units":"deg"}"#
    ) -> JSON {
        JSON.parse(#"{"kind":"object","class":"ItemCamera","index":1,"available":\#(available),"commandsGimbal":\#(commandsGimbal),"cameraAction":\#(action),"gimbalPitch":\#(pitch),"gimbalYaw":\#(yaw)}"#)
    }

    func testAnItemWithNoCameraSectionSaysNothing() {
        XCTAssertNil(itemCameraText(nil))
        XCTAssertNil(itemCameraText(view(available: false)))
    }

    func testAnItemThatCommandsTheGimbalStatesTheActionAndBothAngles() {
        XCTAssertEqual("Take photo · gimbal -45.0 deg / 0.0 deg", itemCameraText(view()))
    }

    func testAnglesAreWithheldForAnItemThatDoesNotCommandTheGimbal() {
        XCTAssertEqual("Take photo", itemCameraText(view(commandsGimbal: false, pitch: "null", yaw: "null")))
    }

    func testAnItemThatTouchesNeitherIsABlankLineRatherThanAnEmptyBullet() {
        XCTAssertNil(itemCameraText(view(commandsGimbal: false, action: "null", pitch: "null", yaw: "null")))
    }

    func testABareNumberIsNotACameraActionAndIsNotDrawnAsOne() {
        XCTAssertNil(itemCameraText(view(commandsGimbal: false, action: #"{"value":0,"text":"0.000","units":""}"#, pitch: "null", yaw: "null")))
        XCTAssertEqual("", namedAction("0.000"))
        XCTAssertEqual("", namedAction("-1"))
        XCTAssertEqual("Take photo", namedAction("Take photo"))
    }

    func testANumberBesideAGimbalAngleStillLeavesTheAngle() {
        XCTAssertEqual("gimbal -45.0 deg / 0.0 deg", itemCameraText(view(action: #"{"value":0,"text":"0.000","units":""}"#)))
    }

    func testAMeasureWithNoUnitsIsNotGivenATrailingSpace() {
        XCTAssertEqual("Take photo", itemCameraText(view(commandsGimbal: false, action: #"{"value":1,"text":"Take photo","units":null}"#, pitch: "null", yaw: "null")))
    }
}

final class CameraChoicesTests: XCTestCase {
    private let enums = #"["No change","Take photo","Take photos (time)","Stop taking photos"]"#

    private func view(_ action: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"ItemCamera","index":2,"available":true,"commandsGimbal":false,"cameraAction":\#(action),"gimbalPitch":null,"gimbalYaw":null}"#)
    }

    func testAFactWithChoicesOffersThemAndSaysWhichIsChosen() throws {
        let choices = try XCTUnwrap(cameraChoices(view(#"{"value":1,"text":"Take photo","units":"","choices":\#(enums),"choice":1}"#)))
        XCTAssertEqual(4, choices.labels.count)
        XCTAssertEqual(1, choices.chosen)
    }

    func testABlankLabelKeepsItsPlaceBecauseChoiceIndexesTheListTheCoreSent() throws {
        let gap = try XCTUnwrap(cameraChoices(view(#"{"value":2,"text":"","units":"","choices":["No change","","Take photo"],"choice":2}"#)))
        XCTAssertEqual("Take photo", gap.labels[gap.chosen])
    }

    func testTheFactMetadataSpellingOffersNothingBecauseTheCoreDoesNotServeIt() {
        XCTAssertNil(cameraChoices(view(#"{"value":1,"text":"Take photo","units":"","enumStrings":\#(enums),"enumIndex":1}"#)))
    }

    func testAPlainNumericFactOffersNothingBecauseItIsNotAChoice() {
        XCTAssertNil(cameraChoices(view(#"{"value":-45.0,"text":"-45.0","units":"deg"}"#)))
        XCTAssertNil(cameraChoices(view(#"{"value":0,"text":"0","units":"","choices":[]}"#)))
        XCTAssertNil(cameraChoices(nil))
    }

    func testTheLabelComesFromTheChoiceListWhenThereIsOne() {
        let named = view(#"{"value":1,"text":"1.000","units":"","choices":\#(enums),"choice":1}"#)
        XCTAssertEqual("Take photo", actionLabel(named))
        XCTAssertEqual("Take photo", itemCameraText(named))
    }

    func testAFactWithNoMetadataKeepsWhateverItsTextCouldRender() {
        XCTAssertEqual("Something", actionLabel(view(#"{"value":3,"text":"Something","units":""}"#)))
    }

    func testAnIndexOutsideTheListFallsBackRatherThanCrashing() {
        let odd = view(#"{"value":9,"text":"9.000","units":"","choices":\#(enums),"choice":9}"#)
        XCTAssertEqual("", actionLabel(odd))
        XCTAssertNil(itemCameraText(odd))
    }
}
