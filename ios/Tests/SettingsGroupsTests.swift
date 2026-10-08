import XCTest
@testable import Aircast

final class SettingsGroupsTests: XCTestCase {
    private func page(_ title: String) -> SettingsPageEntry {
        SettingsPageEntry(title: title, showsLinks: false, showsVideoSources: false, sectionCount: 0)
    }

    private var every: [SettingsPageEntry] {
        ["General", "Remote ID", "Fly View", "ADSB Server", "Video", "Connections", "Something new"].map(page)
    }

    func testEachTabListsItsPagesInDesignOrderAndUnknownPagesLandInGeneral() {
        XCTAssertEqual(tabPages(.Control, every).map(\.title), ["Fly View"])
        XCTAssertEqual(tabPages(.Transmission, every).map(\.title), ["Connections"])
        XCTAssertEqual(tabPages(.General, every).map(\.title), ["General", "Something new"])
    }

    func testSafetyOpensWithTheGuidedLimitsItBorrowsFromFlyView() {
        XCTAssertEqual(tabPages(.Safety, every).map(\.title), ["Fly View", "ADSB Server", "Remote ID"])
    }

    private func block(_ title: String) -> SettingsBlock {
        SettingsBlock(title: title, facts: [
            Fact(
                path: "settings.flyViewSettings.\(title)", name: title, description: "", units: "", valueString: "", value: .number(0),
                enumStrings: [], enumIndex: -1, isBool: true, isString: false, readOnly: false
            ),
        ])
    }

    private var flyView: [SettingsSectionRows] {
        [SettingsSectionRows(title: "Fly View", group: "flyViewSettings", note: "", blocks: [block("Guided Commands"), block("Map and compass")])]
    }

    func testGuidedCommandsMoveToSafetyAndLeaveTheRestOfFlyViewInControl() {
        XCTAssertEqual(sectionsIn(.Safety, "Fly View", flyView).flatMap(\.blocks).map(\.title), ["Guided Commands"])
        XCTAssertEqual(sectionsIn(.Control, "Fly View", flyView).flatMap(\.blocks).map(\.title), ["Map and compass"])
        XCTAssertEqual(sectionsIn(.Camera, "Fly View", flyView), [])
    }

    func testBorrowedBlocksAreHeadedByTheirOwnNames() {
        XCTAssertEqual(borrowedTitle(sectionsIn(.Safety, "Fly View", flyView)), "Guided commands")
    }

    func testEveryKnownPageCarriesItsOwnIconAndUnknownPagesFallBackToSettings() {
        XCTAssertTrue(PAGE_LOOKS.values.allSatisfy { $0.icon != .settings })
        XCTAssertEqual(pageLook("Something new").icon, .settings)
    }
}

final class TabSetupRowsTests: XCTestCase {
    private func component(_ name: String) -> SetupComponent {
        SetupComponent(index: 0, name: name, needsAttention: false)
    }

    func testATabListsOnlyTheSetupPagesTheVehicleReportsInTabOrder() {
        let reported = [component("Flight Modes"), component(SAFETY_SETUP_PAGE), component("Power")]
        XCTAssertEqual(tabSetupComponents(.Safety, reported).map(\.name), [SAFETY_SETUP_PAGE])
        XCTAssertEqual(tabSetupComponents(.Control, reported).map(\.name), [FLIGHT_MODES_PAGE])
        XCTAssertEqual(tabSetupComponents(.Camera, reported), [])
        XCTAssertEqual(tabSetupComponents(.Safety, []), [])
    }

    func testSearchFindsSetupPagesByNameAndNothingForABlankQuery() {
        let reported = [component("Flight Modes"), component("Flight Behavior"), component(SENSORS)]
        XCTAssertEqual(setupSearchHits(reported, "flight").map(\.name), ["Flight Modes", "Flight Behavior"])
        XCTAssertEqual(setupSearchHits(reported, "  "), [])
    }
}

final class ShownSubtitleTests: XCTestCase {
    func testTheDescriptionHidesBehindHelpUntilOpenedAndUnitsStay() {
        XCTAssertEqual(shownSubtitle("Loiter radius · m", "Loiter radius", true, false), ShownSubtitle(text: "m", hasHelp: true))
        XCTAssertEqual(shownSubtitle("Loiter radius · m", "Loiter radius", true, true), ShownSubtitle(text: "Loiter radius · m", hasHelp: true))
        XCTAssertEqual(shownSubtitle("Loiter radius", "Loiter radius", true, false), ShownSubtitle(text: "", hasHelp: true))
    }

    func testADescriptionContainingTheSeparatorStillHidesWhole() {
        XCTAssertEqual(shownSubtitle("Speed · climb · m", "Speed · climb", true, false), ShownSubtitle(text: "m", hasHelp: true))
    }

    func testNoHelpOutsideSettingsWithoutADescriptionOrForACustomSubtitle() {
        XCTAssertEqual(shownSubtitle("Loiter radius · m", "Loiter radius", false, false), ShownSubtitle(text: "Loiter radius · m", hasHelp: false))
        XCTAssertEqual(shownSubtitle("m", "", true, false), ShownSubtitle(text: "m", hasHelp: false))
        XCTAssertEqual(shownSubtitle("Custom", "Loiter radius", true, false), ShownSubtitle(text: "Custom", hasHelp: false))
    }
}
