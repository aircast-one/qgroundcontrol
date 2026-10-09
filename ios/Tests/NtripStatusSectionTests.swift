import XCTest
@testable import Aircast

final class NtripStatusSectionTests: XCTestCase {
    func testCountsAndSizesReadLikeQgc() throws {
        XCTAssertNil(ntripStatus(JSON.parse("{}")))
        let status = try XCTUnwrap(ntripStatus(JSON.parse(#"{"status":"connected","statusMessage":"Connected","messageTypes":[[1005,3],[0,1]],"bytesReceived":2048}"#)))
        XCTAssertEqual(status.messageTypes.map(\.id), [1005, 0])
        XCTAssertEqual(status.messageTypes.map(\.count), [3, 1])
        XCTAssertEqual(dataSize(status.bytesReceived), "2.0 KB")
        XCTAssertEqual(dataSize(512), "512 B")
        XCTAssertEqual(dataSize(1_572_864), "1.5 MB")
        XCTAssertEqual(dataRate(900.0), "900 B/s")
        XCTAssertEqual(dataRate(2048.0), "2.0 KB/s")
    }

    func testTheBrowserListsMountpointsAndMarksTheSelectedOne() {
        let browser = ntripBrowser(JSON.parse(#"{"status":"success","error":"","canBrowse":true,"mountpoints":[{"mountpoint":"NEAR","detail":"RTCM 3.3 · GPS","selected":true}]}"#))
        XCTAssertEqual(browser.mountpoints, [NtripMountpointRow(mountpoint: "NEAR", detail: "RTCM 3.3 · GPS", selected: true)])
        XCTAssertEqual(ntripBrowser(nil).status, "")
    }

    func testTheSameReadingTwiceIsEqualSoThePollDoesNotRedrawTheSection() throws {
        let view = JSON.parse(#"{"status":"connected","messageTypes":[[1005,3],"junk",[1077,9]]}"#)
        let status = try XCTUnwrap(ntripStatus(view))
        XCTAssertEqual(status, ntripStatus(view))
        XCTAssertEqual(status.messageTypes, [NtripMessageType(id: 1005, count: 3), NtripMessageType(id: 1077, count: 9)])
    }
}
