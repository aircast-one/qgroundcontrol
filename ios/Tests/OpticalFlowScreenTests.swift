import XCTest
@testable import Aircast

final class OpticalFlowScreenTests: XCTestCase {
    func testCoreRgbaBytesAreUsedAsIsAndMissingBytesReadAsTransparentBlack() {
        let rgba: [UInt8] = [0x11, 0x22, 0x33, 0xFF, 0x80, 0, 0x40, 0x7F]
        XCTAssertEqual(rgbaPixels(rgba, 2), rgba)
        XCTAssertEqual(rgbaPixels(rgba, 1), [0x11, 0x22, 0x33, 0xFF])
        XCTAssertEqual(rgbaPixels([], 1), [0, 0, 0, 0])
    }

    func testTheFrameDrawsWithItsRedGreenBlueAndAlphaInPlace() throws {
        let image = try XCTUnwrap(bitmapOf(FlowFrame(width: 1, height: 1, rgba: [0x11, 0x22, 0x33, 0xFF]))?.cgImage)
        let context = try XCTUnwrap(CGContext(
            data: nil,
            width: 1,
            height: 1,
            bitsPerComponent: 8,
            bytesPerRow: 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ))
        context.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        let pixel = Array(UnsafeBufferPointer(start: try XCTUnwrap(context.data).assumingMemoryBound(to: UInt8.self), count: 4))
        XCTAssertEqual(pixel, [0x11, 0x22, 0x33, 0xFF])
        XCTAssertNil(bitmapOf(FlowFrame(width: 0, height: 1, rgba: [])))
    }
}
