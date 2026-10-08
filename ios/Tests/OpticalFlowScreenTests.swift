import XCTest
@testable import Aircast

final class OpticalFlowScreenTests: XCTestCase {
    func testCoreRgbaBytesBecomeArgbPixels() {
        let rgba: [UInt8] = [0x11, 0x22, 0x33, 0xFF, 0x80, 0, 0x40, 0x7F]
        XCTAssertEqual(argbPixels(rgba, 2), [0xFF112233, 0x7F800040])
        XCTAssertEqual(argbPixels([], 1), [0], "missing bytes read as transparent black")
    }
}
