import XCTest
@testable import Aircast

final class CameraControlTests: XCTestCase {
    private func camera(_ json: String) -> CameraReading? { cameraReading(JSON.parse(json)) }

    func testACameraWithSeveralStreamsNamesThemAndTheOneShowing() {
        let reading = camera(#"{"present":true,"streamLabels":["Wide","Narrow"],"currentStream":1}"#)!
        XCTAssertEqual(reading.streamLabels, ["Wide", "Narrow"])
        XCTAssertEqual(reading.currentStream, 1)
    }

    func testThePanelOffersTheShuttersTheCoreServesEachWithItsReadout() {
        let panel = camera("""
            {"present":true,"panel":{"visible":true,"inPhotoMode":false,"selectVideoEnabled":true,"selectPhotoEnabled":false,"bothShown":true,
            "video":{"enabled":true,"capturing":true,"idle":false,"clock":"00:01:05"},
            "photo":{"enabled":true,"capturing":true,"idle":false,"press":null,"count":"00042"},
            "freeText":"Free: 12.0 GB","batteryText":null}}
            """)!.panel!
        let video = panel.shutters[0]
        let photo = panel.shutters[1]
        XCTAssertEqual(video.action, CAMERA_RECORD)
        XCTAssertTrue(video.recording && video.readoutActive)
        XCTAssertEqual(video.readout, "00:01:05")
        XCTAssertNil(photo.action, "a single shot in progress has no press")
        XCTAssertEqual(photo.readout, "00042")
        XCTAssertEqual(shutterCaption(panel, video), "Video")
        XCTAssertEqual(shutterReadout(video), "REC 00:01:05")
        XCTAssertEqual(shutterReadout(photo), "00042")
        var idle = video
        idle.readoutActive = false
        XCTAssertNil(shutterReadout(idle), "an idle shutter shows no running clock")
        XCTAssertEqual(panel.freeText, "Free: 12.0 GB")
        XCTAssertNil(panel.batteryText)
        let lapse = camera(#"{"present":true,"panel":{"visible":true,"bothShown":false,"photo":{"enabled":true,"capturing":true,"press":"stop","count":"00003"}}}"#)!.panel!
        XCTAssertEqual(lapse.shutters.count, 1)
        XCTAssertEqual(lapse.shutters[0].action, CAMERA_STOP_PHOTO)
        XCTAssertNil(shutterCaption(lapse, lapse.shutters[0]))
    }

    func testASurveyCameraCanStillBeSwitchedToPhotoBecauseSurveyIsNotPhoto() {
        let survey = camera(#"{"present":true,"mode":2}"#)!
        XCTAssertTrue(modeTapSwitches(survey, false))
        XCTAssertTrue(modeTapSwitches(survey, true))
        XCTAssertFalse(modeTapSwitches(camera(#"{"present":true,"mode":0}"#)!, false))
    }

    func testNoCameraPresentIsNoControls() {
        XCTAssertEqual(camera(#"{"present":true,"labels":["Sony","Thermal"],"selected":1}"#)?.selected, 1)
        XCTAssertNil(camera(#"{"present":true,"labels":[],"selected":null}"#)?.selected)
        XCTAssertNil(cameraReading(nil))
        XCTAssertNil(cameraReading(JSON.parse(#"{"present":false}"#)))
    }
}

final class CameraModeChangeTests: XCTestCase {
    private func view(_ extra: String) -> JSON {
        JSON.parse(#"{"present":true,"hasModes":true,"modeText":"Photo",\#(extra)}"#)
    }

    func testACameraBetweenOperationsMayChangeMode() {
        XCTAssertTrue(cameraReading(view(#""canChangeMode":true"#))!.canChangeMode)
    }

    func testACameraMidOperationMayNotChangeModeEvenThoughItHasModes() {
        let reading = cameraReading(view(#""canChangeMode":false"#))!
        XCTAssertTrue(reading.hasModes)
        XCTAssertFalse(reading.canChangeMode)
    }

    func testAViewThatNeverMentionsCanChangeModeDoesNotInventPermission() {
        XCTAssertFalse(cameraReading(view(#""mode":1"#))!.canChangeMode)
    }
}

final class CameraTimelapseTests: XCTestCase {
    private func view(photoMode: String = #""timelapse""#, lapseSeconds: String = "5.0", lapseCount: String = "10", lapseUnlimited: Bool = false, mode: Int = CAM_MODE_PHOTO) -> JSON {
        JSON.parse("""
            {"kind":"object","class":"Camera","present":true,"hasModes":true,"canChangeMode":true,
             "modeText":"Photo","isRecording":false,"canPhoto":true,"canRecord":true,
             "isTakingPhoto":false,"mode":\(mode),"modeKnown":true,"photoMode":\(photoMode),
             "canStopPhoto":false,"lapseSeconds":\(lapseSeconds),"lapseCount":\(lapseCount),
             "lapseUnlimited":\(lapseUnlimited)}
            """)
    }

    func testASingleShotCameraPlansNothingAndKeepsItsOldButton() {
        XCTAssertNil(lapsePlan(cameraReading(view(photoMode: #""single""#, lapseSeconds: "null", lapseCount: "null"))!))
    }

    func testAShutterThatStartsTenShotsDoesNotSayTakePhoto() {
        XCTAssertEqual(lapsePlan(cameraReading(view())!), "every 5 s, 10 shots")
    }

    func testAnUnlimitedLapseSaysItWillNotStopOnItsOwn() {
        XCTAssertEqual(lapsePlan(cameraReading(view(lapseCount: "0", lapseUnlimited: true))!), "every 5 s, until stopped")
    }

    func testAWholeSecondIntervalIsNotWrittenWithADecimal() {
        XCTAssertEqual(lapsePlan(cameraReading(view())!), "every 5 s, 10 shots")
        XCTAssertEqual(lapsePlan(cameraReading(view(lapseSeconds: "2.5"))!), "every 2.5 s, 10 shots")
    }

    func testAnIntervalBeyondTheIntegerRangeIsWrittenOutRatherThanCrashing() {
        XCTAssertEqual(lapsePlan(cameraReading(view(lapseSeconds: "1e19"))!), "every 10000000000000000000.0 s, 10 shots")
    }
}

final class CameraDetailsTests: XCTestCase {
    private func view(reportsStorage: Bool = true, storageText: String = #""3.2 GB""#, shotsText: String = #""00042""#, batteryText: String = #""87%""#, labels: String = #"["SimCam","Thermal"]"#) -> JSON {
        JSON.parse("""
            {"kind":"object","class":"CameraControl","present":true,"title":"SimCam",
             "labels":\(labels),"stateText":"Idle","reportsStorage":\(reportsStorage),
             "storageText":\(storageText),"shotsText":\(shotsText),"batteryText":\(batteryText),
             "mode":0,"modeKnown":true,"canPhoto":true}
            """)
    }

    func testTheSheetListsWhatTheCameraActuallyReports() {
        let details = cameraDetails(cameraReading(view())!)
        XCTAssertEqual(details.map(\.0), ["State", "Storage", "Photos", "Battery"])
        XCTAssertEqual(details.map(\.1), ["Idle", "3.2 GB", "00042", "87%"])
    }

    func testACameraThatDoesNotReportStorageGetsNoStorageRowRatherThanABlankOne() {
        XCTAssertEqual(cameraDetails(cameraReading(view(reportsStorage: false))!).map(\.0), ["State", "Photos", "Battery"])
    }

    func testACameraWithNoBatteryReadingDropsThatRowToo() {
        XCTAssertEqual(cameraDetails(cameraReading(view(batteryText: #""""#))!).map(\.0), ["State", "Storage", "Photos"])
    }

    func testTheLabelsComeFromTheViewNotFromASecondReadOfTheManager() {
        XCTAssertEqual(cameraReading(view())!.labels, ["SimCam", "Thermal"])
        XCTAssertEqual(cameraReading(view(labels: "[]"))!.labels, [])
    }
}

final class CameraZoomTests: XCTestCase {
    private func camera(hasZoom: Bool = true, level: Double = 50.0) -> CameraReading {
        cameraReading(JSON.parse("""
            {"kind":"object","class":"CameraControl","present":true,"hasModes":true,
             "canChangeMode":true,"modeText":"Photo","isRecording":false,"canPhoto":true,
             "canRecord":true,"isTakingPhoto":false,"mode":0,"modeKnown":true,
             "hasZoom":\(hasZoom),"zoomLevel":\(level)}
            """))!
    }

    func testACameraWithoutZoomOffersNoControlAtAll() {
        XCTAssertNil(zoomText(camera(hasZoom: false)))
    }

    func testTheZoomChipStatesWhereTheZoomIs() {
        XCTAssertEqual(zoomText(camera()), "50%")
    }

    func testAZoomBeyondTheIntegerRangeSaturatesLikeKotlinRatherThanCrashing() {
        XCTAssertEqual(zoomText(camera(level: 1e300)), "\(Int.max)%")
        XCTAssertEqual(zoomText(camera(level: -1e300)), "\(Int.min)%")
    }
}
