import SwiftUI
import XCTest
@testable import Aircast

final class PlanControlsTests: XCTestCase {
    private func resolved(_ colour: Color) -> Color.Resolved { colour.resolve(in: EnvironmentValues()) }

    private func assertColour(_ red: Float, _ green: Float, _ blue: Float, _ opacity: Float, _ colour: Color, file: StaticString = #filePath, line: UInt = #line) {
        let shown = resolved(colour)
        XCTAssertEqual(red, shown.red, accuracy: 0.01, file: file, line: line)
        XCTAssertEqual(green, shown.green, accuracy: 0.01, file: file, line: line)
        XCTAssertEqual(blue, shown.blue, accuracy: 0.01, file: file, line: line)
        XCTAssertEqual(opacity, shown.opacity, accuracy: 0.01, file: file, line: line)
    }

    func testASixDigitColourIsOpaque() {
        XCTAssertEqual(resolved(Color(hex: 0xFFB300)), resolved(hexColour("#FFB300")))
        XCTAssertEqual(resolved(Color(hex: 0xFFB300)), resolved(hexColour("FFB300")))
        assertColour(1, 1, 1, 1, hexColour("#FFFFFF"))
    }

    func testAnEightDigitColourCarriesItsAlphaInTheLeadingByte() {
        assertColour(1, 1, 1, Float(0x80) / 255, hexColour("#80FFFFFF"))
    }

    func testAColourThatIsNotHexOrHasTheWrongLengthFallsBackToGray() {
        XCTAssertEqual(resolved(.gray), resolved(hexColour("#ZZZZZZ")))
        XCTAssertEqual(resolved(.gray), resolved(hexColour("#FFF")))
        XCTAssertEqual(resolved(.gray), resolved(hexColour("")))
    }

    private let chips = [CGSize(width: 40, height: 20), CGSize(width: 50, height: 30), CGSize(width: 30, height: 10)]

    func testChipsWrapOntoANewLineOnceTheNextOneWouldOverflow() {
        XCTAssertEqual([[0, 1], [2]], flowLines(chips, 100, spacing: 8))
        XCTAssertEqual([[0, 1, 2]], flowLines(chips, 136, spacing: 8))
        XCTAssertEqual([[0], [1], [2]], flowLines(chips, 10, spacing: 8))
        XCTAssertEqual([], flowLines([], 100, spacing: 8))
    }

    func testTheRowIsAsTallAsItsTallestChipPerLinePlusTheGapsBetweenLines() {
        XCTAssertEqual(CGSize(width: 100, height: 30 + 4 + 10), flowSize(chips, 100, spacing: 8, lineSpacing: 4))
        XCTAssertEqual(CGSize(width: 136, height: 30), flowSize(chips, nil, spacing: 8, lineSpacing: 4))
        XCTAssertEqual(.zero, flowSize([], nil, spacing: 8, lineSpacing: 4))
    }

    func testChipsSitLeftToRightAndAreCentredVerticallyInTheirLine() {
        XCTAssertEqual(
            [CGPoint(x: 0, y: 5), CGPoint(x: 48, y: 0), CGPoint(x: 0, y: 34)],
            flowPositions(chips, 100, spacing: 8, lineSpacing: 4, centred: false)
        )
    }

    func testACentredRowSplitsEachLinesSlackEvenly() {
        XCTAssertEqual(
            [CGPoint(x: 1, y: 5), CGPoint(x: 49, y: 0), CGPoint(x: 35, y: 34)],
            flowPositions(chips, 100, spacing: 8, lineSpacing: 4, centred: true)
        )
    }
}
