import XCTest
@testable import Aircast

final class NativeNavigationTests: XCTestCase {
    func testARequestedPageIsPushedAboveItsTabUnlessTheTabDrawsItInline() {
        XCTAssertEqual(requestedRoutes("Connections", .Transmission), [.Page("Connections")])
        XCTAssertEqual(requestedRoutes("Fly View", .Control), [])
        XCTAssertEqual(requestedRoutes(nil, .General), [])
        XCTAssertEqual(requestedRoutes("Connections", .General), [])
    }

    func testSetupCountsAsShowingWhereverItSitsInTheStack() {
        XCTAssertFalse(setupShowing([]))
        XCTAssertFalse(setupShowing([.Page("Connections")]))
        XCTAssertTrue(setupShowing([.Setup]))
        XCTAssertTrue(setupShowing([.Page("Maps"), .Setup]))
    }

    func testTheDeviceTabOpensDevicePagesAndTheOtherTabsSettingsPages() {
        XCTAssertEqual(tabRoute(.Device, "/flights"), .Device("/flights"))
        XCTAssertEqual(tabRoute(.Transmission, "Connections"), .Page("Connections"))
    }

    func testTheDeviceStackIsTheDevicePagesInTheStackInOrder() {
        XCTAssertEqual(deviceHrefs([.Device("/connect"), .Device("/streaming")]), ["/connect", "/streaming"])
        XCTAssertEqual(deviceHrefs([.Page("Maps"), .Setup]), [])
    }

    func testOpeningADevicePageFromAnotherReplacesTheTopAndReturnsThere() {
        let path: [SettingsRoute] = [.Device("/connect"), .Device("/streaming")]
        let opened = withDeviceHrefs(path, deviceRoutesOpenedFrom(deviceHrefs(path), "/settings/camera?camera=cam", "/streaming?camera=cam"))
        XCTAssertEqual(opened, [.Device("/connect"), .Device("/streaming?camera=cam"), .Device("/settings/camera?camera=cam")])
    }

    func testABlockedPageRefusesBackBeforeItsOwnStepBack() {
        XCTAssertEqual(pageBack(nil, false), .System)
        XCTAssertEqual(pageBack(nil, true), .Step)
        XCTAssertEqual(pageBack(CALIBRATION_BLOCK, true), .Refused(CALIBRATION_BLOCK))
        XCTAssertEqual(pageBack(LOG_DOWNLOAD_BLOCK, false), .Refused(LOG_DOWNLOAD_BLOCK))
    }

    func testPagesBehindRowsShareASectionAndInlinePagesStandApart() {
        let page = { (title: String) in SettingsPageEntry(title: title, showsLinks: false, showsVideoSources: false, sectionCount: 1) }
        let pages = ["Maps", "3D Viewer", "Fly View", "Plan View", "Flight Modes"].map(page)
        let runs = pageRuns(pages) { $0.title.hasSuffix("View") }
        XCTAssertEqual(runs.map(\.inline), [false, true, true, false])
        XCTAssertEqual(runs.map { $0.pages.map(\.title) }, [["Maps", "3D Viewer"], ["Fly View"], ["Plan View"], ["Flight Modes"]])
        XCTAssertEqual(Set(runs.map(\.id)).count, runs.count)
        XCTAssertEqual(pageRuns([]) { _ in true }, [])
    }
}
