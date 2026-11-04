import XCTest
@testable import Aircast

final class ShellPortraitTests: XCTestCase {
    func testOnlyAPhoneShapedIosWindowTakesThePortraitLayout() {
        let tall = CGSize(width: 800, height: 1200)
        let wide = CGSize(width: 1280, height: 800)
        XCTAssertFalse(shellIsPortrait(wide))
        #if os(macOS)
        XCTAssertFalse(shellIsPortrait(tall), "a Mac window dragged taller than wide flipped to the phone layout")
        #else
        XCTAssertTrue(shellIsPortrait(tall))
        #endif
    }
}

#if os(macOS)
final class MacCommandsTests: XCTestCase {
    private func offer(_ id: String, _ state: String = "ready", title: String = "", confirm: Bool = true) -> GuidedOffer {
        GuidedOffer(id: id, title: title, offer: state, reason: "", prompt: "", destructive: false, carriesValue: false, confirm: confirm)
    }

    func testTheFirstNineTabsTakeCommandDigits() {
        XCTAssertEqual(tabShortcutKey(0), "1")
        XCTAssertEqual(tabShortcutKey(2), "3")
        XCTAssertEqual(tabShortcutKey(8), "9")
        XCTAssertNil(tabShortcutKey(9))
        XCTAssertNil(tabShortcutKey(-1))
        XCTAssertEqual(visibleTabs(true).indices.compactMap(tabShortcutKey), ["1", "2", "3"])
    }

    func testTheVehicleMenuKeepsItsItemsGreyedWithoutAVehicle() {
        let items = menuFlightItems([:], false)
        XCTAssertEqual(items.map(\.id), ["rtl", "land", PAUSE])
        XCTAssertEqual(items.map(\.title), ["Return\u{2026}", "Land\u{2026}", "Pause\u{2026}"])
        XCTAssertFalse(items.contains(where: \.enabled))
    }

    func testAMenuFlightActionIsOnlyEnabledWhileTheVehicleOffersIt() {
        let offers = ["rtl": offer("rtl"), "land": offer("land", "disabled"), PAUSE: offer(PAUSE, "hidden")]
        XCTAssertEqual(menuFlightItems(offers, true).map(\.enabled), [true, false, false])
        XCTAssertFalse(menuFlightItems(offers, false).contains(where: \.enabled), "an offer from a vehicle that has dropped stayed pickable")
    }

    func testTheMenuNamesAnActionAsTheCoreDoesAndMarksOnesThatAsk() {
        let offers = ["rtl": offer("rtl", title: "Return to Launch"), "land": offer("land", title: "Land", confirm: false), PAUSE: offer(PAUSE, title: "Hold", confirm: false)]
        XCTAssertEqual(menuFlightItems(offers, true).map(\.title), ["Return to Launch\u{2026}", "Land", "Hold\u{2026}"])
    }

    func testEscClosesSettingsBeforeLeavingFullScreenVideo() {
        XCTAssertEqual(shellEscape(settingsOpen: true, fullScreen: true), .CloseSettings)
        XCTAssertEqual(shellEscape(settingsOpen: false, fullScreen: true), .LeaveFullScreen)
        XCTAssertNil(shellEscape(settingsOpen: false, fullScreen: false))
    }

    func testTheWindowNeverStartsOrShrinksPhoneShaped() {
        XCTAssertGreaterThanOrEqual(MAC_WINDOW_DEFAULT_WIDTH, MAC_WINDOW_MIN_WIDTH)
        XCTAssertGreaterThanOrEqual(MAC_WINDOW_DEFAULT_HEIGHT, MAC_WINDOW_MIN_HEIGHT)
        XCTAssertGreaterThan(MAC_WINDOW_MIN_WIDTH, MAC_WINDOW_MIN_HEIGHT)
    }
}
#endif
