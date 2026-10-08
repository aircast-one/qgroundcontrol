import XCTest
@testable import Aircast

final class TrackingDragTests: XCTestCase {
    private let letterboxed = PaintedRect(left: 100, top: 0, width: 400, height: 300)

    private func box(_ request: TrackingRequest?) -> TrackingBox? {
        if case .Box(let rect) = request { return rect }
        return nil
    }

    func testADragIsMeasuredAgainstThePictureNotTheSurface() {
        let rect = box(trackingRequest(200, 60, 300, 150, letterboxed))!
        XCTAssertEqual(rect.x, 0.25, accuracy: 1e-9, "an x of 200 on a picture starting at 100 is a quarter across the image")
        XCTAssertEqual(rect.y, 0.2, accuracy: 1e-9)
        XCTAssertEqual(rect.width, 0.25, accuracy: 1e-9)
        XCTAssertEqual(rect.height, 0.3, accuracy: 1e-9)
    }

    func testADragBackwardsIsTheSameRectangleAsADragForwards() {
        XCTAssertEqual(trackingRequest(300, 150, 200, 60, letterboxed), trackingRequest(200, 60, 300, 150, letterboxed))
    }

    func testATapIsAPointWithARadiusNotARectangleWithNoArea() {
        guard case .Point(let x, let y, let radius) = trackingRequest(300, 150, 303, 152, letterboxed) else {
            return XCTFail("Qt sends the point message when the drag is under ten pixels in both axes")
        }
        XCTAssertEqual(x, 0.5, accuracy: 1e-9)
        XCTAssertEqual(y, 0.5, accuracy: 1e-9)
        XCTAssertEqual(radius, 0.125, accuracy: 1e-9, "the radius is a fraction of the picture's width")
    }

    func testADragBeyondThePictureIsClampedToIt() {
        let rect = box(trackingRequest(-500, -500, 5000, 5000, letterboxed))!
        XCTAssertEqual(rect.x, 0, accuracy: 1e-9)
        XCTAssertEqual(rect.y, 0, accuracy: 1e-9)
        XCTAssertEqual(rect.width, 1, accuracy: 1e-9)
        XCTAssertEqual(rect.height, 1, accuracy: 1e-9)
    }

    func testNoPictureIsNothingToAimAt() {
        XCTAssertNil(trackingRequest(1, 1, 2, 2, PaintedRect(left: 0, top: 0, width: 0, height: 0)))
    }

    func testThePointPayloadCarriesTheRadiusBesideThePointSoArityPicksTheOverload() {
        let sent = trackingPointObject(0.5, 0.25)
        XCTAssertEqual(Set(sent.keys), ["x", "y"])
        XCTAssertEqual(sent["x"]!, 0.5, accuracy: 1e-9)
        XCTAssertEqual(sent["y"]!, 0.25, accuracy: 1e-9)
    }

    func testADragThatNeverTouchesThePictureAsksForNothing() {
        XCTAssertNil(trackingRequest(20, 60, 80, 150, letterboxed), "both corners clamp to the same edge, which is a rectangle with no area")
        XCTAssertNil(trackingRequest(600, 60, 900, 150, letterboxed))
        XCTAssertNil(trackingRequest(200, -400, 300, -200, letterboxed))
    }

    func testADragFromOutsideThatEndsInsideStillHasAreaAndIsKept() {
        let rect = box(trackingRequest(20, 60, 300, 150, letterboxed))!
        XCTAssertEqual(rect.x, 0, accuracy: 1e-9)
        XCTAssertEqual(rect.width, 0.5, accuracy: 1e-9)
    }
}
