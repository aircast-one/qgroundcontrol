import XCTest
@testable import Aircast

final class OfflineStatusTests: XCTestCase {
    func testTheOfflinePageReadsTheCoresFootnoteAndLinks() {
        let status = offlineStatus(JSON.parse(
            #"{"title":"Connection Failed","footnote":"Couldn't connect to Drone.","busy":false,"noLinks":false,"editAddress":true,"links":[{"index":2,"text":"Connect to Drone","description":"TCP\nrefused","retry":true,"failed":true,"connected":false}]}"#
        ))
        XCTAssertEqual(status?.title, "Connection Failed")
        XCTAssertEqual(status?.editAddress, true)
        XCTAssertEqual(status?.links, [OfflineLink(index: 2, text: "Connect to Drone", description: "TCP\nrefused", retry: true, failed: true, connected: false)])
        XCTAssertNil(offlineStatus(JSON.parse("{}")))
    }
}
