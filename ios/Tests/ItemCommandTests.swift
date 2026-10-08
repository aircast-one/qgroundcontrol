import XCTest
@testable import Aircast

final class ItemCommandTests: XCTestCase {
    func testTheEditorNoteIsTheCommandDescriptionUnlessRawEditIsOnLikeSimpleItemEditor() {
        let view = JSON.parse(#"{"commandDescription":"Travel to a position in 3D space."}"#)
        XCTAssertEqual("Travel to a position in 3D space.", itemNote(view, rawOn: false))
        XCTAssertEqual(RAW_EDIT_NOTE, itemNote(view, rawOn: true))
        XCTAssertNil(itemNote(JSON.parse(#"{"commandDescription":null}"#), rawOn: false))
    }

    func testATakeoffItemOffersNoCommandChangeLikeMissionItemEditor() {
        XCTAssertEqual(
            [true, false, false],
            [#"{"simple":true,"takeoff":false}"#, #"{"simple":true,"takeoff":true}"#, #"{"simple":false}"#].map { commandEditable(JSON.parse($0)) }
        )
    }

    func testThePickerOpensOnTheItemsCategoryLikeMissionCommandDialog() {
        XCTAssertEqual("Advanced", startCategory(["Basic", "Advanced"], "Advanced"))
        XCTAssertEqual("Basic", startCategory(["Basic", "Advanced"], "Gone"))
        XCTAssertEqual("Basic", startCategory(["Basic", "Advanced"], nil))
    }

    func testAPlanesNewTakeoffShowsTheClimbOutStepUntilDoneLikeSimpleItemEditorsWizard() {
        let lines = wizardLines(JSON.parse(#"{"wizardMode":true,"wizardText":["Move 'T' Takeoff to the climbout location.","Ensure clear of obstacles and into the wind."]}"#))
        XCTAssertEqual(["Move 'T' Takeoff to the climbout location.", "Ensure clear of obstacles and into the wind."], lines)
        XCTAssertEqual([], wizardLines(JSON.parse(#"{"wizardMode":false,"wizardText":["x"]}"#)))
        XCTAssertEqual("plan.missionController.visualItems.1.wizardMode", wizardModePath(1))
    }
}
