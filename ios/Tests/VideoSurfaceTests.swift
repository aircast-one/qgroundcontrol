import XCTest
@testable import Aircast

final class VideoSurfaceTests: XCTestCase {
    func testADecodedFrameNeedsRowsWideEnoughForEveryPixel() throws {
        XCTAssertNil(DecodedFrame(width: 16, height: 2, stride: 63))
        XCTAssertNil(DecodedFrame(width: 0, height: 2, stride: 64))
        XCTAssertNil(DecodedFrame(width: 16, height: 0, stride: 64))
        XCTAssertNil(DecodedFrame(width: 16, height: 2, stride: -64))
        let frame = try XCTUnwrap(DecodedFrame(width: 16, height: 2, stride: 128))
        XCTAssertEqual(frame.byteCount, 256)
        XCTAssertTrue(frame.fits(256))
        XCTAssertFalse(frame.fits(255))
    }

    func testTheCopyBufferRoundsEachRowUpToTheSixtyFourByteAlignment() {
        XCTAssertEqual(DecodedFrame.capacity(width: 17, height: 2), 256)
        XCTAssertEqual(DecodedFrame.capacity(width: 16, height: 2), 128)
        XCTAssertEqual(DecodedFrame.capacity(width: 0, height: 2), 0)
    }
}
