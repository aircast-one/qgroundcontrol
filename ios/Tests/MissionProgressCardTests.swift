import XCTest
@testable import Aircast

final class MissionProgressCardTests: XCTestCase {
    func testTheCardReadsTheCoresProgressAndOffersTheNextWaypointUntilTheLast() {
        let shown = #"{"shown":true,"current":3,"last":6,"fraction":0.5,"distanceToNext":"84","distanceUnits":"m","canSkip":true,"skipTo":4}"#
        let progress = missionProgress(JSON.parse(shown))!
        XCTAssertEqual(missionProgressLine(progress), "To waypoint 3 of 6 · 84 m")
        XCTAssertEqual(progress.skipTo, 4)
        XCTAssertNil(missionProgress(JSON.parse(shown.replacingOccurrences(of: #""shown":true"#, with: #""shown":false"#))))
        XCTAssertNil(missionProgress(JSON.parse(#"{"shown":true,"current":6,"last":6,"canSkip":false,"skipTo":6}"#))!.skipTo)
    }
}
