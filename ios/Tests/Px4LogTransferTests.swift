import XCTest
@testable import Aircast

final class Px4LogTransferTests: XCTestCase {
    func testThePageReadsTheLoggingStateSettingsAndSavedFiles() throws {
        let log = try XCTUnwrap(mavlinkLog(JSON.parse(#"{"vehiclePx4":true,"logRunning":false,"canStartLog":true,"persistence":true,"settings":{"emailAddress":"a@b.c","windSpeed":"5"},"files":[{"name":"001-x.ulg","size":2048,"uploaded":true}],"uploading":false,"uploadingFile":null,"message":null}"#)))
        XCTAssertTrue(log.canStart)
        XCTAssertEqual(log.files, [LogFile(name: "001-x.ulg", size: 2048, uploaded: true)])
        XCTAssertEqual(log.settings["emailAddress"].string, "a@b.c")
        XCTAssertEqual(WIND_SPEEDS.first { $0.1 == log.settings["windSpeed"].string }?.0, "Breeze")
        XCTAssertEqual(log.uploadProgress, 0)
    }

    func testTheUploadProgressIsTheCoresSentFraction() {
        XCTAssertEqual(mavlinkLog(JSON.parse(#"{"uploading":true,"uploadingFile":"001-x","uploadProgress":0.4}"#))?.uploadProgress, 0.4)
    }

    func testTheSavedFileButtonsWaitWhileLoggingOrUploadingLikePX4LogTransferSettingsIdle() {
        let idle = { (running: Bool, uploading: Bool) in
            logListIdle(mavlinkLog(JSON.parse(#"{"logRunning":\#(running),"uploading":\#(uploading)}"#))!)
        }
        XCTAssertTrue(idle(false, false))
        XCTAssertFalse(idle(true, false))
        XCTAssertFalse(idle(false, true))
    }

    func testUploadUsesTheEmailTypedButNotYetSavedAndSavesItFirstLikeSaveItems() {
        let settings = JSON.parse(#"{"emailAddress":"","description":"d"}"#)
        let drafts = ["emailAddress": "a@b.c", "description": "d"]
        XCTAssertEqual(uploadEmail(settings, drafts), "a@b.c")
        XCTAssertEqual(unsavedTexts(settings, drafts).map(\.0), ["emailAddress"])
        XCTAssertEqual(unsavedTexts(settings, drafts).map(\.1), ["a@b.c"])
    }

    func testUploadFallsBackToTheSavedEmailWhenNothingWasTyped() {
        let settings = JSON.parse(#"{"emailAddress":"a@b.c"}"#)
        XCTAssertEqual(uploadEmail(settings, [:]), "a@b.c")
        XCTAssertTrue(unsavedTexts(settings, [:]).isEmpty)
    }
}
