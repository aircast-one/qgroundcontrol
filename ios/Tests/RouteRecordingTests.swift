import XCTest
@testable import Aircast

final class RouteRecordingTests: XCTestCase {
    private func view(_ fields: String) -> JSON { JSON.parse(#"{"kind":"object","class":"RouteRecorder",\#(fields)}"#) }

    func testARecordingShowsTheCoresPillTextAndNoSavedNotice() {
        let recording = routeRecorder(view(#""available":true,"recording":true,"text":"Recording route · 123 m","savedFile":null,"savedName":null,"message":null"#))!
        XCTAssertTrue(recording.recording)
        XCTAssertEqual(recording.text, "Recording route · 123 m")
        XCTAssertNil(recording.message)
        XCTAssertNil(routeRecorder(JSON.parse(#"{"class":"Orbit"}"#)))
    }

    func testAStoppedRecordingKeepsTheSavedRouteUntilItIsDismissed() {
        let saved = routeRecorder(view(#""available":true,"recording":false,"text":null,"savedFile":"/m/Route 2026-10-24 10-00-00.kml","savedName":"Route 2026-10-24 10-00-00","message":"Route saved""#))!
        XCTAssertNil(saved.text)
        XCTAssertEqual(saved.savedFile, "/m/Route 2026-10-24 10-00-00.kml")
        XCTAssertEqual(saved.savedName, "Route 2026-10-24 10-00-00")
        XCTAssertEqual(saved.message, "Route saved")
        let tooShort = routeRecorder(view(#""available":true,"recording":false,"savedFile":null,"message":"Route not saved: the vehicle didn't fly far enough while recording.""#))!
        XCTAssertNil(tooShort.savedFile)
    }

    func testARefusedRenameKeepsTheOldFileAndSaysTheNameIsTaken() {
        let taken = renamed(JSON.parse(#"{"ok":false,"refusal":"nameUsed","reason":"Name already used","path":"/m/a.kml"}"#), "/m/a.kml")
        XCTAssertEqual(taken, Renamed(file: "/m/a.kml", message: "Name already used"))
        XCTAssertEqual(renamed(JSON.parse(#"{"ok":true,"path":"/m/Tower 7.kml"}"#), "/m/a.kml"), Renamed(file: "/m/Tower 7.kml", message: nil))
    }

    func testTheFlyControlsOfferRecordOrStopAndNothingWithoutAVehicle() {
        let idle = routeTile(RouteRecorder(available: true, recording: false, text: nil, savedFile: nil, savedName: nil, message: nil), { _ in })!
        XCTAssertEqual(idle.label, "Record route")
        XCTAssertFalse(idle.warning)
        let running = routeTile(RouteRecorder(available: true, recording: true, text: "Recording route · 5 m", savedFile: nil, savedName: nil, message: nil), { _ in })!
        XCTAssertEqual(running.label, "Stop recording")
        XCTAssertNil(routeTile(RouteRecorder(available: false, recording: false, text: nil, savedFile: nil, savedName: nil, message: nil)) { _ in })
        XCTAssertNil(routeTile(nil) { _ in })
    }
}
