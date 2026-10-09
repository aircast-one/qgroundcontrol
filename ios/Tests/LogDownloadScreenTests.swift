import XCTest
@testable import Aircast

private func localDate(_ year: Int, _ month: Int, _ day: Int, _ hour: Int = 0, _ minute: Int = 0, _ second: Int = 0) -> Date {
    Calendar.current.date(from: DateComponents(year: year, month: month, day: day, hour: hour, minute: minute, second: second))!
}

final class LogDownloadScreenTests: XCTestCase {
    private let served = #"""
        {"connected":true,"busy":false,"canRefresh":true,"canDownload":true,
         "canCancel":false,"canErase":true,"anyDownloaded":true,
         "emptyText":"","eraseWarning":"This erases every log on the vehicle.",
         "entries":[
           {"index":0,"id":1,"sizeBytes":4096,"sizeText":"4.0KB","status":"Downloaded",
            "received":true,"selected":false,"time":"2026-09-08T01:20:00",
            "timeState":"known"},
           {"index":1,"id":2,"sizeBytes":10240,"sizeText":"10.0KB","status":"Available",
            "received":true,"selected":true,"time":"2026-09-08T02:20:00",
            "timeState":"known"}]}
        """#

    func testLogsSplitIntoTheVehiclesAndThePhonesByStatusId() throws {
        let saved = served.replacingOccurrences(of: #""status":"Downloaded","#, with: #""status":"Downloaded","statusId":"downloaded","#)
        let sections = logSections(try XCTUnwrap(logsView(JSON.parse(saved))).entries)

        XCTAssertEqual(sections.map(\.0), ["On the vehicle", "On this phone"])
        XCTAssertEqual(sections.map { $0.1.map(\.id) }, [[2], [1]])
        XCTAssertEqual(logSections(try XCTUnwrap(logsView(JSON.parse(served))).entries).map(\.0), ["On the vehicle"])

        let rows = logRows(try XCTUnwrap(logsView(JSON.parse(saved))).entries)
        XCTAssertEqual(rows.map(\.entry.id), [2, 1])
        XCTAssertEqual(rows.map(\.header), ["On the vehicle", "On this phone"])
    }

    func testTheDownloadCardNamesTheLogBeingFetchedByItsStatusIdNotItsTranslatedStatus() throws {
        let downloading = served.replacingOccurrences(of: #""status":"Available","#, with: #""status":"1.2 MB (300 KB/s)","statusId":"downloading","#)
        let card = downloadCard(try XCTUnwrap(logsView(JSON.parse(downloading))).entries)
        XCTAssertEqual(card.0, "Downloading log 2")
        XCTAssertEqual(card.1, "1.2 MB (300 KB/s)")

        let idle = downloadCard(try XCTUnwrap(logsView(JSON.parse(served))).entries)
        XCTAssertEqual(idle.0, "Downloading")
        XCTAssertEqual(idle.1, "")
    }

    func testTheEntriesCarryTheCoresFormattedSizeAndTime() throws {
        let logs = try XCTUnwrap(logsView(JSON.parse(served)))

        XCTAssertEqual(logs.entries.map(\.sizeStr), ["4.0KB", "10.0KB"])
        XCTAssertTrue(logs.entries.allSatisfy { !$0.time.isBlank && $0.time != "Date unknown" })
        XCTAssertEqual(logs.entries.map(\.id), [1, 2])
        XCTAssertEqual(logs.entries.map(\.status), ["Downloaded", "Available"])
    }

    func testTheButtonsFollowTheCoreRatherThanTheHeadsOwnArithmetic() throws {
        let logs = try XCTUnwrap(logsView(JSON.parse(served)))

        XCTAssertTrue(logs.canRefresh)
        XCTAssertTrue(logs.canDownload)
        XCTAssertFalse(logs.canCancel)
        XCTAssertTrue(logs.anyDownloaded)
        XCTAssertEqual(logs.eraseWarning, "This erases every log on the vehicle.")
    }

    func testAVehicleWithNoLogsIsConnectedWithAnEmptyList() throws {
        let logs = try XCTUnwrap(logsView(JSON.parse(#"{"connected":true,"entries":[],"emptyText":"This vehicle reports no flight logs."}"#)))

        XCTAssertTrue(logs.connected)
        XCTAssertEqual(logs.entries, [])
        XCTAssertEqual(logs.emptyText, "This vehicle reports no flight logs.")
    }

    func testNoViewAtAllIsNoScreenState() {
        XCTAssertNil(logsView(nil))
    }

    func testOpeningThePageRefreshesTheListOfAConnectedVehicleUnlessATransferIsRunning() {
        XCTAssertTrue(shouldAutoRefreshLogs(true, false))
        XCTAssertFalse(shouldAutoRefreshLogs(true, true))
        XCTAssertFalse(shouldAutoRefreshLogs(false, false))
    }
}

final class LogTimeTextTests: XCTestCase {
    private let fixed: (Date) -> String = { _ in "FORMATTED" }

    func testAnUnreceivedEntryShowsNoTime() {
        XCTAssertEqual(logTimeText("2026-09-10T12:00:00Z", TIME_UNRECEIVED, fixed), "")
    }

    func testAVehicleWithAnUnsetClockSaysSoRatherThanShowing1970() {
        XCTAssertEqual(logTimeText("1970-01-01T00:00:00Z", TIME_UNKNOWN, fixed), "Date unknown")
    }

    func testAKnownTimeIsHandedToTheLocalFormatter() {
        XCTAssertEqual(logTimeText("2026-09-10T12:00:00Z", "known", fixed), "FORMATTED")
    }

    func testAnUnparseableTimeDoesNotCrashTheList() {
        XCTAssertEqual(logTimeText("not a time", "known", fixed), "Date unknown")
    }

    func testAQtLocalTimeWithNoZoneDesignatorIsWhatTheBridgeActuallySends() {
        let seen = logTimeText("2026-09-08T01:20:00", "known") { String($0.timeIntervalSince1970) }
        XCTAssertEqual(seen, String(localDate(2026, 9, 8, 1, 20, 0).timeIntervalSince1970))
    }

    func testAnOffsetFormIsConvertedRatherThanRejected() {
        XCTAssertEqual(logTimeText("2026-09-08T01:20:00+03:00", "known", fixed), "FORMATTED")
    }
}

final class LogTimeZoneTests: XCTestCase {
    func testAZoneLessTimeIsWallClockAndMustNotBeReadAsUTC() {
        XCTAssertEqual(logLocalTime("2026-09-08T14:42:51"), localDate(2026, 9, 8, 14, 42, 51))
    }

    func testAnOffsetTimeIsConvertedIntoTheReadersZone() {
        let expected = ISO8601DateFormatter().date(from: "2026-09-08T14:42:51+04:00")
        XCTAssertEqual(logLocalTime("2026-09-08T14:42:51+04:00"), expected)
    }
}

final class LogSavePathTests: XCTestCase {
    private func logs(_ anyDownloaded: Bool, _ savePath: String, _ reason: String) -> LogsView? {
        logsView(JSON.parse(#"{"kind":"object","class":"LogDownload","entries":[],"busy":false,"anyDownloaded":\#(anyDownloaded),"savePath":\#(savePath),"savePathReason":\#(reason)}"#))
    }

    func testNothingDownloadedSaysNothingAboutWhereItWent() {
        XCTAssertNil(savedToText(logs(false, #""/sdcard/Logs""#, "null")))
        XCTAssertNil(savedToText(nil))
    }

    func testAKnownDirectoryIsNamed() {
        XCTAssertEqual(savedToText(logs(true, #""/sdcard/Logs""#, "null")), "Saved to /sdcard/Logs")
    }

    func testAnEmptyPathWithAReasonSaysTheReasonBecauseTheTwoEmptiesDiffer() {
        XCTAssertEqual(
            savedToText(logs(true, #""""#, #""The folder chosen for logs is no longer there.""#)),
            "The folder chosen for logs is no longer there."
        )
    }

    func testAnEmptyPathWithNoReasonStaysSilentRatherThanPrintingSavedToNowhere() {
        XCTAssertNil(savedToText(logs(true, #""""#, "null")))
    }

    func testATimestampThatParsesAsNeitherShapeIsNullRatherThanAWrongDate() {
        XCTAssertNil(logLocalTime(""))
        XCTAssertNil(logLocalTime("not a date"), "a made-up date must not be shown beside a real log")
    }

    func testALogTimeReadsCompactlyLikeThePenpotList() {
        let today = localDate(2026, 10, 3)
        XCTAssertEqual(compactLogTime(localDate(2026, 10, 2, 9, 0), today), "Yesterday")
        XCTAssertEqual(compactLogTime(localDate(2026, 9, 8, 2, 20), today), "Sep 8")
        XCTAssertEqual(compactLogTime(localDate(2025, 9, 8, 2, 20), today), "Sep 8, 2025")
        XCTAssertFalse(compactLogTime(localDate(2026, 10, 3, 14, 2), today).contains("Oct"))
    }
}
