import XCTest
@testable import Aircast

final class TabReselectTests: XCTestCase {
    func testReSelectingAnalyzeDropsItsSubPage() {
        XCTAssertTrue(reselectClearsAnalyze(.Analyze, .Analyze))
    }

    func testReSelectingAnotherTabLeavesTheAnalyzeSubPageAlone() {
        Tab.allCases.filter { $0 != .Analyze }.forEach { entry in
            XCTAssertFalse(reselectClearsAnalyze(entry, entry), "re-tapping \(entry) cleared a sub-page belonging to a tab the operator is not in")
        }
    }

    func testSwitchingToADifferentTabIsNotAReSelectionAtAll() {
        XCTAssertFalse(reselectClearsAnalyze(.Plan, .Analyze))
        XCTAssertFalse(reselectClearsAnalyze(.Analyze, .Plan))
    }

    func testAnalyzeLeavesTheNavigationWhileAdvancedModeIsOffLikeSelectViewDropdown() {
        XCTAssertFalse(visibleTabs(false).contains(.Analyze))
        XCTAssertEqual(visibleTabs(true), Tab.allCases)
    }

    func testANoticeDestinationNamesATabInAnyCase() {
        XCTAssertEqual(Tab.from("PLAN"), .Plan)
        XCTAssertEqual(Tab.from("fly"), .Fly)
        XCTAssertNil(Tab.from("setup"))
    }
}
