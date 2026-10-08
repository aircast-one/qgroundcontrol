import XCTest
@testable import Aircast

final class ComplexPatternTests: XCTestCase {
    private func items(_ json: String) -> [MissionItem] { allMissionItems(JSON.parse(json)) }

    func testACorridorScanIsAPatternEvenThoughItIsNotASurvey() {
        let plan = items(#"{"kind":"object","items":[{"index":0,"sequence":0,"name":"Waypoint","kind":"waypoint","simple":true},{"index":1,"sequence":1,"name":"Survey","kind":"survey","simple":false},{"index":2,"sequence":2,"name":"Corridor Scan","kind":"corridor","simple":false},{"index":3,"sequence":3,"name":"Structure Scan","kind":"structure","simple":false}]}"#)
        XCTAssertFalse(plan[0].complexPattern)
        XCTAssertEqual([1, 2, 3], plan.filter(\.complexPattern).map(\.index))
    }

    func testAnItemTheCoreSaidNothingAboutIsTreatedAsSimpleRatherThanAskedAbout() {
        let plan = items(#"{"kind":"object","items":[{"index":0,"sequence":0,"name":"Waypoint","kind":"waypoint"}]}"#)
        XCTAssertEqual(1, plan.count)
        XCTAssertFalse(plan[0].complexPattern)
    }

    func testTheCameraQuestionIsAskedOfEveryPatternNotOnlyTheOneNamedSurvey() {
        let plan = items(#"{"kind":"object","items":[{"index":2,"sequence":2,"name":"Corridor Scan","kind":"corridor","simple":false,"cameraShots":27}]}"#)
        XCTAssertTrue(plan[0].complexPattern, "a corridor inherits cameraCalc from TransectStyleComplexItem, so gating on the survey type hides camera work it really did")
    }
}
