import SwiftUI
import XCTest
@testable import Aircast

@MainActor
final class AircastSheetTests: XCTestCase {
    func testReopeningWhileClosingKeepsTheEntryWanted() {
        let stack = SheetStack()
        let sheet = UUID()
        stack.request(sheet, 0, dialog: false)
        stack.appeared(sheet)
        stack.release(sheet)
        stack.request(sheet, 0, dialog: false)
        stack.disappeared(sheet)
        XCTAssertEqual(stack.entries.map(\.id), [sheet])
        XCTAssertFalse(stack.entries[0].shown)
    }

    func testAUserDismissalForgetsTheSheet() {
        let stack = SheetStack()
        let sheet = UUID()
        stack.request(sheet, 0, dialog: false)
        stack.appeared(sheet)
        stack.disappeared(sheet)
        XCTAssertTrue(stack.entries.isEmpty)
    }

    func testAHostThatGoesAwayLeavesAShownSheetToClose() {
        let stack = SheetStack()
        let waiting = UUID()
        let shown = UUID()
        stack.request(shown, 0, dialog: false)
        stack.appeared(shown)
        stack.request(waiting, 0, dialog: false)
        stack.abandon(waiting)
        stack.abandon(shown)
        XCTAssertEqual(stack.entries.map(\.id), [shown])
        stack.disappeared(shown)
        XCTAssertTrue(stack.entries.isEmpty)
    }

    func testASheetHandedToANewHostOnRotationLeavesNoStaleLevel() {
        let stack = SheetStack()
        let portrait = UUID()
        let landscape = UUID()
        stack.request(portrait, 0, dialog: false)
        stack.appeared(portrait)
        stack.request(landscape, 0, dialog: false)
        stack.abandon(portrait)
        stack.moved(portrait, landscape, 0, dialog: false)
        XCTAssertEqual(stack.entries.map(\.id), [landscape])
        XCTAssertEqual(stack.level(dialogs: true), 1)
        stack.disappeared(landscape)
        XCTAssertEqual(stack.level(dialogs: true), 0)
    }

    func testAppDialogsFollowTheTopmostOpenSheet() {
        let stack = SheetStack()
        let more = UUID()
        let complete = UUID()
        XCTAssertEqual(stack.level(dialogs: true), 0)
        stack.request(more, 0, dialog: false)
        XCTAssertEqual(stack.level(dialogs: true), 0, "not on screen until it appears")
        stack.appeared(more)
        XCTAssertEqual(stack.level(dialogs: true), 1)
        stack.request(complete, 1, dialog: true)
        stack.appeared(complete)
        XCTAssertEqual(stack.level(dialogs: true), 2, "alerts go above the mission-complete sheet")
        XCTAssertEqual(stack.level(dialogs: false), 1, "the mission-complete sheet stays where it was opened")
        stack.release(more)
        XCTAssertEqual(stack.level(dialogs: false), 0, "a closing sheet no longer hosts dialogs")
    }

    func testTheGimbalQuestionGoesBeforeQueuedMessages() {
        let message = AppMessage(title: "Reboot", text: "Reboot the vehicle?")
        XCTAssertEqual(appAlert(true, [message]), .Gimbal)
        XCTAssertEqual(appAlert(false, [message]), .Message(message))
        XCTAssertNil(appAlert(false, []))
    }
}
