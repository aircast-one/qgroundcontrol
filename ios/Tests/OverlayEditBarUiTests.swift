import SwiftUI
import XCTest
@testable import Aircast

private let PHONE_WIDTH: CGFloat = 360
private let LANDSCAPE_BAR_WIDTH: CGFloat = 600
private let BAR_PADDING: CGFloat = 8

@MainActor
final class OverlayEditBarUiTests: XCTestCase {
    private func editingLayout() -> OverlayLayoutState {
        let layout = OverlayLayoutState(overlayLayoutStore())
        layout.reset()
        layout.editing = true
        return layout
    }

    private func measured(_ view: some View, _ width: CGFloat? = nil) -> CGSize {
        UIHostingController(rootView: view.font(.labelLarge)).sizeThatFits(in: CGSize(width: width ?? 10_000, height: 10_000))
    }

    private func itemSizes() -> [CGSize] {
        ["Readout size: Default", "Battery bar: Shown", "Reset positions and hidden items"].map { measured(Button($0) {}.buttonStyle(.text)) }
            + [measured(Button("Done") {}.buttonStyle(.filled))]
    }

    private func bar(inFlight: Bool, _ width: CGFloat) -> CGSize {
        measured(OverlayEditBarContent(layout: editingLayout(), inFlight: inFlight).frame(width: width), width)
    }

    func testOnAPhoneHeldUprightTheEditBarKeepsDoneAndEveryOptionWholeOnScreen() {
        let inner = PHONE_WIDTH - BAR_PADDING * 2
        let items = itemSizes()
        items.forEach { XCTAssertLessThanOrEqual($0.width, inner) }
        let oneLineEach = flowSize(items, inner, spacing: Space.s2, lineSpacing: 0).height + BAR_PADDING
        XCTAssertEqual(bar(inFlight: false, PHONE_WIDTH).height, oneLineEach, accuracy: 0.5)
    }

    func testDoneKeepsToTheEndOfItsLineOnAWideBar() {
        let inner = LANDSCAPE_BAR_WIDTH - BAR_PADDING * 2
        let items = itemSizes()
        let placed = lastAtLineEnd(flowPositions(items, inner, spacing: Space.s2, lineSpacing: 0, centred: false), items, inner)
        XCTAssertEqual(placed.last!.x + items.last!.width, inner, accuracy: 0.5)
        XCTAssertEqual(placed.dropLast(), Array(flowPositions(items, inner, spacing: Space.s2, lineSpacing: 0, centred: false).dropLast()))
    }

    func testInFlightTheEditBarSaysWhyAndOnTheGroundItDoesNot() {
        XCTAssertGreaterThan(bar(inFlight: true, PHONE_WIDTH).height, bar(inFlight: false, PHONE_WIDTH).height + 20)
    }

    func testWhileAFlightDecisionIsOpenTheEditBarStepsAside() {
        let flyScreen = FlyScreenState(layout: editingLayout())
        flyScreen.guidedPanels = 1
        XCTAssertEqual(measured(OverlayEditBar().environment(flyScreen), PHONE_WIDTH).height, 0)
        flyScreen.guidedPanels = 0
        XCTAssertGreaterThan(measured(OverlayEditBar().environment(flyScreen), PHONE_WIDTH).height, 0)
    }
}
