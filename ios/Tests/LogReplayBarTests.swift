import XCTest
@testable import Aircast

final class LogReplayBarTests: XCTestCase {
    func testTheBarReadsTheCoreReplayState() throws {
        XCTAssertNil(logReplay(JSON.parse(#"{"available":false}"#)))
        let replay = try XCTUnwrap(logReplay(JSON.parse(#"{"available":true,"shown":true,"loaded":true,"playing":false,"percent":12.5,"playheadTime":"01m:15s","totalTime":"10m:00s","speedIndex":4,"speeds":["0.1","0.25","0.5","1x","2x","5x","10x"],"canLoad":false,"loadRefusal":"x","error":""}"#)))
        XCTAssertEqual(replay.speeds[replay.speedIndex], "2x")
        XCTAssertEqual(replay.percent, 12.5)
        XCTAssertFalse(replay.playing)
    }

    func testTheBarReadsItsPositionAndSpeedsLikeTheDesign() throws {
        let replay = try XCTUnwrap(logReplay(JSON.parse(#"{"available":true,"shown":true,"playheadTime":"01m:15s","totalTime":"10m:00s","speeds":["0.25","2x"]}"#)))
        XCTAssertEqual(replayProgress(replay), "01m:15s of 10m:00s")
        var blank = replay
        blank.playheadTime = ""
        blank.totalTime = ""
        XCTAssertEqual(replayProgress(blank), "")
        XCTAssertEqual(replay.speeds.map(speedLabel), ["0.25×", "2×"])
    }

    func testTheReplayLinkIsNamedAfterThePickedFileLikeLinkManager() {
        XCTAssertEqual(replayFileName("2026-10-03 10-11-12.tlog"), "2026-10-03 10-11-12.tlog")
        XCTAssertEqual(replayFileName("../../flight.tlog"), "flight.tlog")
        XCTAssertEqual(replayFileName(nil), "log-replay.tlog")
        XCTAssertEqual(replayFileName(".."), "log-replay.tlog")
    }

    func testTouchingTheThumbWithoutMovingItDoesNotSeek() {
        var scrub = ReplayScrub()
        XCTAssertNil(scrub.edit(true))
        XCTAssertNil(scrub.edit(false))
        XCTAssertEqual(scrub.shown(12), 12)
    }

    func testADragShowsTheThumbAndSeeksOnceWhenReleased() {
        var scrub = ReplayScrub()
        XCTAssertNil(scrub.edit(true))
        XCTAssertNil(scrub.move(30))
        XCTAssertNil(scrub.move(40))
        XCTAssertEqual(scrub.shown(12), 40)
        XCTAssertEqual(scrub.edit(false), 40)
        XCTAssertEqual(scrub.shown(12), 12)
        XCTAssertNil(scrub.edit(true))
        XCTAssertNil(scrub.edit(false), "a later touch does not replay the last drag")
    }

    func testAnAdjustmentOutsideADragSeeksAtOnceAndDoesNotFreezeTheBar() {
        var scrub = ReplayScrub()
        XCTAssertEqual(scrub.move(55), 55)
        XCTAssertEqual(scrub.shown(12), 12)
    }
}
