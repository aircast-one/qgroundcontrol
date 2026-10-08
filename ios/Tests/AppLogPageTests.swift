import XCTest
@testable import Aircast

final class AppLogPageTests: XCTestCase {
    func testSearchTextIsEncodedSoCommasStayInsideOneArgument() {
        XCTAssertEqual(
            appLogPath(AppLogFilter(levelIndex: 2, category: "hub", text: "a, b", regex: true), 7),
            "view.appLog(1,hub,a%2C+b,1,7)"
        )
        XCTAssertEqual(appLogPath(AppLogFilter(levelIndex: 0), nil), "view.appLog(0,,,0,)")
    }

    func testHeldEntriesEvictedFromTheCoreAreDroppedBeforeNewOnesAreAppended() throws {
        let read = try XCTUnwrap(appLogRead(JSON.parse(
            #"{"class":"AppLog","levels":[],"categories":[],"first":5,"entries":["#
                + #"{"sequence":6,"level":2,"message":"W lost","category":"hub","timestamp":"03:20:11.000","source":""}]}"#
        )))
        let held = [Int64(4), 5].map { AppLogEntry(sequence: $0, level: 1, message: "I", category: "hub", timestamp: "", source: "") }
        XCTAssertEqual(mergedEntries(held, read).map(\.sequence), [5, 6])
    }

    func testTheSaveNameAndTypeFollowTheLogSaveFormatSettingLikeTheQgcDialogSuffix() {
        XCTAssertEqual(appLogFileName(JSON.parse(#"{"value":0}"#)), "QGCConsole.txt")
        XCTAssertEqual(appLogFileName(JSON.parse(#"{"value":1}"#)), "QGCConsole.csv")
        XCTAssertEqual(appLogFileName(nil), "QGCConsole.txt")
        XCTAssertEqual(appLogMime("QGCConsole.csv"), "text/csv")
        XCTAssertEqual(appLogMime("QGCConsole.txt"), "text/plain")
    }
}
