import XCTest
@testable import Aircast

final class DestructiveActionsTests: XCTestCase {
    private func served(_ entries: String...) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Camera","present":true,"destructiveActions":[\#(entries.joined(separator: ","))]}"#)
    }

    private func action(_ id: String, _ offer: String, reason: String = "", prompt: String = "P") -> String {
        #"{"id":"\#(id)","label":"L\#(id)","button":"B\#(id)","title":"\#(id)","prompt":"\#(prompt)","offer":"\#(offer)","reason":"\#(reason)","destructive":true}"#
    }

    func testAHiddenActionIsNotDrawnAtAll() {
        XCTAssertEqual(destructiveActions(served(action("formatStorage", "hidden"), action("resetSettings", "ready"))).map(\.id), ["resetSettings"])
    }

    func testABlockedActionIsDrawnDisabledWithTheReasonTheCoreGave() {
        let blocked = destructiveActions(served(action("formatStorage", "blocked", reason: "The camera is recording.")))[0]
        XCTAssertFalse(blocked.ready, "the row stays, so the operator sees why rather than why not")
        XCTAssertTrue(blocked.blocked)
        XCTAssertEqual(destructiveReasonFor(blocked), "The camera is recording.")
    }

    func testAReadyActionCarriesNoReasonToShow() {
        let ready = destructiveActions(served(action("resetSettings", "ready")))[0]
        XCTAssertTrue(ready.ready)
        XCTAssertNil(destructiveReasonFor(ready), "a reason beneath an enabled button reads as a warning about the action rather than an explanation of why it cannot be used")
    }

    func testTheConfirmationBodyIsTheCoresSentence() {
        let reset = destructiveActions(served(action("resetSettings", "ready", prompt: "Put every camera setting back to its factory value. This cannot be undone.")))[0]
        XCTAssertEqual(reset.prompt, "Put every camera setting back to its factory value. This cannot be undone.")
    }

    func testTheRowLabelAndButtonTextAreTheCoresApartFromTheDialogTitle() {
        let reset = destructiveActions(served(action("resetSettings", "ready")))[0]
        XCTAssertEqual([reset.label, reset.button, reset.title], ["LresetSettings", "BresetSettings", "resetSettings"])
    }

    func testTheSheetsVideoRowsCarryTheCameraDialogsOwnTitles() {
        XCTAssertEqual(CAMERA_SHEET_VIDEO_SETTINGS.map(\.value), ["Video Grid Lines", "Video Screen Fit"])
    }

    func testAnActionThisHeadCannotSendIsNotGivenAPath() {
        XCTAssertEqual(destructiveInvokePath("resetSettings"), CAMERA_RESET)
        XCTAssertEqual(destructiveInvokePath("formatStorage"), CAMERA_FORMAT)
        XCTAssertNil(destructiveInvokePath("somethingLater"))
    }

    func testACoreTooOldToServeThemLeavesTheSheetWithoutTheRows() {
        XCTAssertEqual(destructiveActions(nil), [])
        XCTAssertEqual(destructiveActions(JSON.parse(#"{"kind":"object","class":"Camera"}"#)), [])
    }
}
