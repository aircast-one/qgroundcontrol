import XCTest
@testable import Aircast

final class LogReplayNameTests: XCTestCase {
    func testTheReplayLinkIsNamedAfterThePickedFileLikeLinkManager() {
        XCTAssertEqual(replayFileName("2026-10-03 10-11-12.tlog"), "2026-10-03 10-11-12.tlog")
        XCTAssertEqual(replayFileName("../../flight.tlog"), "flight.tlog")
        XCTAssertEqual(replayFileName(nil), "log-replay.tlog")
        XCTAssertEqual(replayFileName(".."), "log-replay.tlog")
    }
}
