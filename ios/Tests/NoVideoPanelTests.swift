import XCTest
@testable import Aircast

final class NoVideoPanelTests: XCTestCase {
    private let video = VideoReading(available: true, decoding: false, sourceSize: nil, summary: "Waiting for a stream.", activeSource: 0, cameras: [], noVideoText: "No video on UDP port 5600")

    func testTheDetailNamesTheSourceAndHowLongItHasFailed() {
        XCTAssertEqual(noVideoDetail(video, 12), "No video on UDP port 5600 for 12 s")
        var streaming = video
        streaming.streaming = true
        XCTAssertEqual(noVideoDetail(streaming, 125), "Receiving data \u{2014} waiting for video for 2 min")
        XCTAssertEqual(elapsedText(3900), "1 h 5 min")
    }

    func testASetUpCameraShowsItsOwnStateWhileNoAircraftIsConnected() {
        var unavailable = video
        unavailable.available = false
        XCTAssertFalse(looksForAircraft(connected: false, video: video))
        XCTAssertTrue(looksForAircraft(connected: false, video: unavailable))
        XCTAssertTrue(looksForAircraft(connected: false, video: nil))
        XCTAssertFalse(looksForAircraft(connected: true, video: nil))
    }

    func testAStalledStreamSaysWhyInsteadOfHowLong() {
        let reason = "No answer from 192.168.1.50:8889. Check the address and that this device is on the drone's network."
        var stalled = video
        stalled.noVideoReason = reason
        XCTAssertEqual(noVideoDetail(stalled, 70), reason)
    }

    func testThePanelReadsTheActiveCamerasOwnStatusLikeFlightDisplayViewVideo() {
        var failing = video
        failing.cameras = [VideoCamera(slot: 0, status: "Connection failed, retrying", configured: true)]
        XCTAssertEqual(activeCameraStatus(failing), "Connection failed, retrying")
        XCTAssertNil(activeCameraStatus(video))
    }

    private func unavailable(sourceChosen: Bool, configured: Bool, streamEnabled: Bool = true) -> VideoReading {
        VideoReading(
            available: false, decoding: false, sourceSize: nil, summary: "No stream URL is set.", activeSource: 0,
            cameras: [VideoCamera(slot: 0, status: "", configured: configured)],
            streamEnabled: streamEnabled,
            sourceChosen: sourceChosen
        )
    }

    func testAnUnavailableStreamSaysWhatIsMissingAndWhereToFixIt() {
        XCTAssertEqual(unavailableVideoState(unavailable(sourceChosen: false, configured: false))?.action, .SetUp)
        XCTAssertEqual(unavailableVideoState(unavailable(sourceChosen: true, configured: false))?.title, "No stream address")
        XCTAssertEqual(unavailableVideoState(unavailable(sourceChosen: true, configured: true))?.action, .Settings)
        var available = unavailable(sourceChosen: true, configured: true)
        available.available = true
        XCTAssertNil(unavailableVideoState(available))
    }

    func testAStreamSwitchedOffOffersToTurnItOnRatherThanSendingThePilotToSettings() {
        let off = unavailableVideoState(unavailable(sourceChosen: true, configured: true, streamEnabled: false))
        XCTAssertEqual(off?.title, "Video off")
        XCTAssertEqual(off?.action, .TurnOn)
        XCTAssertEqual(unavailableVideoState(unavailable(sourceChosen: false, configured: false, streamEnabled: false))?.action, .TurnOn)
    }

    func testWhileArmedOnlyTheOneTapTurnOnSurvivesNeverATripToSettings() {
        let off = whileArmed(unavailableVideoState(unavailable(sourceChosen: true, configured: true, streamEnabled: false))!)
        XCTAssertEqual(off.action, .TurnOn)
        XCTAssertEqual(off.detail, "")
        XCTAssertEqual(whileArmed(unavailableVideoState(unavailable(sourceChosen: true, configured: false))!).action, NoVideoAction.None)
        XCTAssertEqual(whileArmed(unavailableVideoState(unavailable(sourceChosen: false, configured: false))!).action, NoVideoAction.None)
    }

    func testTheMissingVideoCopyPointsAtTheVideoSourcesPageOnTheTransmissionTab() {
        XCTAssertEqual(unavailableVideoState(unavailable(sourceChosen: false, configured: false))?.detail, "Add a camera in Settings \u{203A} Transmission \u{203A} Video sources.")
        XCTAssertEqual(unavailableVideoState(unavailable(sourceChosen: true, configured: false))?.detail, "Enter the stream address in Settings \u{203A} Transmission \u{203A} Video sources.")
    }

    func testTheVideoButtonIsNamedForTheVideoSourcesPageItOpens() {
        let navigation = AppNavigationState()
        let button = videoSourcesButton(navigation)
        button.onClick()
        XCTAssertEqual(button.label, "Video sources")
        XCTAssertEqual(navigation.settingsPage, VIDEO_SOURCES_PAGE)
    }

    private func rect(_ left: CGFloat, _ top: CGFloat, _ right: CGFloat, _ bottom: CGFloat) -> CGRect {
        CGRect(x: left, y: top, width: right - left, height: bottom - top)
    }

    func testTheMessageTakesTheLargestFreeSideOfTheInstrumentsOrTheWholeAreaWhenTheyAreElsewhere() {
        let free = rect(0, 100, 1000, 1500)
        XCTAssertEqual(messageRegion(free, [rect(0, 100, 500, 220)], 10, 200, 50), rect(0, 230, 1000, 1500))
        XCTAssertEqual(messageRegion(free, [rect(0, 100, 300, 1500)], 10, 200, 50), rect(310, 100, 1000, 1500))
        XCTAssertEqual(messageRegion(free, [rect(300, 1600, 700, 1700)], 10, 200, 50), free)
        XCTAssertEqual(messageRegion(free, [], 10, 200, 50), free)
    }

    func testTheMessageClearsEveryWidgetLookingPastTheFirstSideItFinds() {
        let free = rect(0, 100, 1000, 600)
        XCTAssertEqual(messageRegion(free, [rect(0, 100, 400, 200), rect(0, 210, 900, 600)], 10, 200, 50), rect(410, 100, 1000, 200))
    }

    func testWithNoGapBigEnoughForThePillThereIsNoMessageRatherThanASqueezedOne() {
        XCTAssertNil(messageRegion(rect(0, 100, 1000, 600), [rect(0, 100, 950, 600)], 10, 200, 50))
        XCTAssertNil(messageRegion(rect(0, 0, 150, 600), [], 10, 200, 50))
    }
}

final class CentredRegionTests: XCTestCase {
    private func rect(_ left: CGFloat, _ top: CGFloat, _ right: CGFloat, _ bottom: CGFloat) -> CGRect {
        CGRect(x: left, y: top, width: right - left, height: bottom - top)
    }

    func testAMessageBesideTheMiniMapStillCentresOnTheScreenWhenItFits() {
        XCTAssertEqual(centredRegion(rect(410, 100, 1990, 900), 1000, 400), rect(410, 100, 1590, 900))
    }

    func testARoomTooLopsidedToCentreInKeepsItsOwnShape() {
        let room = rect(900, 100, 1990, 900)
        XCTAssertEqual(centredRegion(room, 1000, 400), room)
    }
}
