import XCTest
@testable import Aircast

final class TrackingBoxTests: XCTestCase {
    private func camera(supported: Bool = true, active: Bool = true, rect: String = #"{"x":0.35,"y":0.30,"width":0.30,"height":0.40}"#, shapes: String = #"["rectangle","point"]"#) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Camera","present":true,"tracking":{"supported":\#(supported),"requested":\#(active),"reported":\#(active),"shapes":\#(shapes),"rect":\#(rect)}}"#)
    }

    func testACameraThatCannotTrackOffersNothing() {
        XCTAssertNil(trackingReading(camera(supported: false)))
        XCTAssertNil(trackingReading(JSON.parse(#"{"kind":"object","class":"Camera"}"#)))
        XCTAssertNil(trackingReading(nil))
    }

    func testTheBoxIsTheServedRectangle() {
        let box = trackingReading(camera())!.box!
        XCTAssertEqual(box.x, 0.35, accuracy: 1e-9)
        XCTAssertEqual(box.y, 0.30, accuracy: 1e-9)
        XCTAssertEqual(box.width, 0.30, accuracy: 1e-9)
        XCTAssertEqual(box.height, 0.40, accuracy: 1e-9)
    }

    func testARectangleWithNoAreaIsNotABox() {
        XCTAssertNil(trackingReading(camera(rect: #"{"x":0.5,"y":0.5,"width":0.0,"height":0.0}"#))!.box, "a zero-sized rect would draw as a frame the operator cannot see and cannot dismiss")
        XCTAssertNil(trackingReading(camera(rect: "null"))!.box)
    }

    func testStoppingIsOfferedOnlyWhileSomethingIsBeingTracked() {
        XCTAssertTrue(trackingCanStop(trackingReading(camera(active: true))))
        XCTAssertFalse(trackingCanStop(trackingReading(camera(active: false, rect: "null"))))
        XCTAssertFalse(trackingCanStop(nil))
    }

    func testStartingNeedsAShapeTheCameraAccepts() {
        XCTAssertTrue(trackingCanStart(trackingReading(camera(active: false, rect: "null"))))
        XCTAssertFalse(trackingCanStart(trackingReading(camera(active: true))), "already tracking, so the next command is stop")
        XCTAssertFalse(trackingCanStart(trackingReading(camera(active: false, rect: "null", shapes: "[]"))), "a camera claiming tracking but naming no shape cannot be told how to start")
    }

    func testTheRectanglePayloadIsTheShapeTheBridgeNowAccepts() {
        let sent = trackingRectObject(TRACKING_CENTRE)
        XCTAssertEqual(Set(sent.keys), ["x", "y", "width", "height"])
        XCTAssertEqual(sent["x"]!, 0.4, accuracy: 1e-9)
        XCTAssertEqual(sent["y"]!, 0.4, accuracy: 1e-9)
        XCTAssertEqual(sent["width"]!, 0.2, accuracy: 1e-9)
        XCTAssertEqual(sent["height"]!, 0.2, accuracy: 1e-9)
    }

    func testADragAimsOnlyOnceTrackingHasBeenArmedAsTheQtViewDoes() {
        XCTAssertFalse(trackingCanAim(trackingReading(camera(active: false))), "FlyViewVideo.qml:157 creates an ROI only when trackingEnabled; a drag before that arms nothing")
        XCTAssertTrue(trackingCanAim(trackingReading(camera(active: true))))
        XCTAssertFalse(trackingCanAim(trackingReading(camera(active: true, shapes: "[]"))))
        XCTAssertFalse(trackingCanAim(nil))
    }

    func testTheToggleSaysWhatTheNextTapDoes() {
        XCTAssertEqual(trackingToggleLabel(trackingReading(camera(active: false))!), "Track something")
        XCTAssertEqual(trackingToggleLabel(trackingReading(camera(active: true))!), "Stop tracking")
    }
}
