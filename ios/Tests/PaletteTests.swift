import Observation
import UIKit
import XCTest
@testable import Aircast

final class PaletteTests: XCTestCase {
    func testIndoorIsDarkOutdoorIsLightSystemFollowsTheDevice() {
        XCTAssertTrue(paletteIsDark(1, false))
        XCTAssertFalse(paletteIsDark(0, true))
        XCTAssertTrue(paletteIsDark(2, true))
        XCTAssertFalse(paletteIsDark(2, false))
    }

    func testSystemLeavesTheWindowToTheDeviceSoLaterAppearanceChangesShow() {
        XCTAssertEqual(paletteWindowStyle(true, true), .unspecified)
        XCTAssertEqual(paletteWindowStyle(true, false), .unspecified)
        XCTAssertEqual(paletteWindowStyle(false, true), .dark)
        XCTAssertEqual(paletteWindowStyle(false, false), .light)
    }

    func testDarkWhileTheSettingIsUnread() {
        XCTAssertTrue(paletteIsDark(nil, false))
    }

    func testDefaultsToDarkOnceAndOnlyWhenUntouched() {
        XCTAssertTrue(shouldDefaultToDark(false, false))
        XCTAssertFalse(shouldDefaultToDark(true, false))
        XCTAssertFalse(shouldDefaultToDark(false, true))
    }

    func testThePaletteChoiceIsNamedDarkAndLightByItsStoredValueAsPenpotNamesIt() {
        let fact = Fact(path: PALETTE_SETTING, name: "indoorPalette", description: "Color Scheme", units: "", valueString: "1", value: .number(1),
                        enumStrings: ["Innen", "Außen", "System"], enumValues: ["1", "0", "2"], enumIndex: 0, isBool: false, isString: false, readOnly: false)
        XCTAssertEqual(paletteNamed(fact).enumStrings, ["Dark", "Light", "System"])
        var other = fact
        other.path = "settings.x"
        other.enumStrings = ["A", "B"]
        other.enumValues = ["1", "0"]
        XCTAssertEqual(paletteNamed(other).enumStrings, ["A", "B"])
    }

    @MainActor
    func testGivingUpTheDarkDefaultRedrawsWhatReadIt() {
        let awaited = PaletteDefault.shared
        awaited.awaiting = true
        let redrawn = expectation(description: "readers redraw")
        withObservationTracking { _ = awaited.awaiting } onChange: { redrawn.fulfill() }
        awaited.awaiting = false
        wait(for: [redrawn], timeout: 1)
    }
}
