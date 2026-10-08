import XCTest
@testable import Aircast

final class DebugUiTests: XCTestCase {
    private func refusal(_ result: Result<DebugCommand, DebugUiError>) -> String? {
        guard case .failure(let error) = result else { return nil }
        return error.message
    }

    func testEachCommandParsesToWhatTheAppApplies() {
        XCTAssertEqual(try debugCommand("tab", "plan").get(), .ShowTab(.Plan))
        XCTAssertEqual(try debugCommand("fly-view", "Video").get(), .ShowFlyView(.Video))
        XCTAssertEqual(try debugCommand("orientation", "landscape").get(), .Orient(.landscapeRight))
        XCTAssertEqual(try debugCommand("layout-edit", "off").get(), .EditLayout(false))
        XCTAssertEqual(try debugCommand("layout-reset", nil).get(), .ResetLayout)
        XCTAssertEqual(try debugCommand("open", "takeoff").get(), .Open("takeoff"))
    }

    func testABadCommandNamesWhatItAcceptsInsteadOfGuessing() {
        XCTAssertEqual(refusal(debugCommand("rotate", nil)), "cmd must be one of \(DEBUG_COMMANDS)")
        XCTAssertEqual(refusal(debugCommand("orientation", "sideways")), "orientation must be one of landscape, reverse-landscape, portrait, auto")
        XCTAssertEqual(refusal(debugCommand("tab", "Home")), "tab must be one of Fly, Plan, Analyze")
        XCTAssertEqual(
            refusal(debugCommand("open", "")),
            "open needs a sheet (more, readings, camera, gimbal, status, status-all, modes, traffic, settings) or a flight action id, e.g. takeoff, rtl, land"
        )
    }
}
