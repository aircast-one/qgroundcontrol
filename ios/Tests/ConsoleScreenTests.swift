import SwiftUI
import XCTest
@testable import Aircast

final class ConsoleScreenTests: XCTestCase {
    func testAnEmptyConsoleShowsTheBarePromptLikeTheMAVLinkConsolePageTextArea() {
        XCTAssertEqual(CONSOLE_EMPTY_TEXT, "> ")
    }

    func testOpeningThePageAsksTheCoreForAFreshController() {
        XCTAssertEqual(CONSOLE_OPEN, "mavlinkConsole.open")
    }
}

final class ConsoleFollowTailTests: XCTestCase {
    func testAnEmptyListFollowsTheTail() {
        XCTAssertTrue(shouldFollowTail(nil, 0))
    }

    func testAReaderAtTheBottomKeepsFollowing() {
        XCTAssertTrue(shouldFollowTail(9, 10))
    }

    func testAReaderOneLineBehindStillFollowsSoANewLineDoesNotStrandThem() {
        XCTAssertTrue(shouldFollowTail(9, 11))
    }

    func testAReaderScrolledUpIsLeftWhereTheyAre() {
        XCTAssertFalse(shouldFollowTail(3, 40))
    }

    func testScrollingUpByTwoLinesIsEnoughToStopTheYank() {
        XCTAssertFalse(shouldFollowTail(7, 10))
    }
}

private struct Span: Equatable {
    let start: Int
    let end: Int
    let color: Color
}

private func spans(_ styled: AttributedString) -> [Span] {
    styled.runs.compactMap { run in
        run[AttributeScopes.SwiftUIAttributes.ForegroundColorAttribute.self].map { color in
            Span(
                start: styled.characters.distance(from: styled.startIndex, to: run.range.lowerBound),
                end: styled.characters.distance(from: styled.startIndex, to: run.range.upperBound),
                color: color
            )
        }
    }
}

final class ConsoleLinesTests: XCTestCase {
    func testLinesComeFromTheServedArray() {
        XCTAssertEqual(consoleLines(JSON.parse(#"{"lines":["a","b"]}"#)), ["a", "b"])
        XCTAssertEqual(consoleLines(JSON.parse("{}")), [])
        XCTAssertEqual(consoleLines(nil), [])
    }

    func testALeadingWarnOrErrorIsColouredAsMAVLinkConsoleControllerMarksIt() {
        let warn = Color.yellow
        let error = Color.red
        XCTAssertEqual(spans(consoleLineStyled("WARN  [sensors] no baro", warn, error)), [Span(start: 0, end: 4, color: warn)])
        XCTAssertEqual(spans(consoleLineStyled("ERROR x", warn, error)), [Span(start: 0, end: 5, color: error)])
        XCTAssertTrue(spans(consoleLineStyled("warn: an INFO WARN", warn, error)).isEmpty, "case sensitive, and only at the start")
    }

    func testPromptLinesAreTheEchoedCommands() {
        XCTAssertTrue(isPromptLine("nsh> free"))
        XCTAssertFalse(isPromptLine("total  used  free"))
        XCTAssertFalse(isPromptLine("note: nsh> appears mid-line"))
    }
}

final class ConsolePasteTests: XCTestCase {
    func testAPastedBlockSendsItsCompleteLinesAndKeepsTheUnfinishedTailLikeHandleClipboard() {
        let (sent, left) = splitCompleteLines(TextFieldValue("ver all\nfree\nto", TextRange(15)))
        XCTAssertEqual(sent, "ver all\nfree")
        XCTAssertEqual(left.text, "to")
        XCTAssertEqual(left.selection, TextRange(2))
    }

    func testTextAfterTheCursorStaysBehindThePastedTail() {
        let (sent, left) = splitCompleteLines(TextFieldValue("lsfree\nps\n -l", TextRange(10)))
        XCTAssertEqual(sent, "lsfree\nps")
        XCTAssertEqual(left.text, " -l")
        XCTAssertEqual(left.selection, TextRange(0))
    }

    func testTypingWithoutANewlineSendsNothing() {
        let field = TextFieldValue("help", TextRange(4))
        let (sent, left) = splitCompleteLines(field)
        XCTAssertNil(sent)
        XCTAssertEqual(left, field)
    }

    func testTheReturnKeySendsTheWholeLineLikeTheImeSendAction() {
        XCTAssertTrue(typedReturn("ls -l", "ls\n -l"))
        XCTAssertTrue(typedReturn("ls", "ls\n"))
        XCTAssertTrue(typedReturn("", "\n"))
        XCTAssertFalse(typedReturn("ls", "ls\nps\n"))
        XCTAssertFalse(typedReturn("ls", "ls -"))
    }
}
