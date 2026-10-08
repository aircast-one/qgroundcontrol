import XCTest
@testable import Aircast

final class PlanFilesViewTests: XCTestCase {
    func testPatternsAreOfferedByTheCanonicalNameTheInsertTakes() {
        let view = JSON.parse(#"{"patterns":[{"name":"Survey","title":"Survey"},{"name":"Corridor Scan","title":"Korridor-Scan"},{"name":"","title":"x"}]}"#)
        XCTAssertEqual(["Survey", "Corridor Scan"], patternNames(view))
        XCTAssertEqual([], patternNames(nil))
    }

    func testThePlanFileIsTheWholePathAndNoneIsNull() {
        XCTAssertEqual("/plans/ridge.plan", currentPlanPath(JSON.parse(#"{"filePath":"/plans/ridge.plan"}"#)))
        XCTAssertNil(currentPlanPath(JSON.parse(#"{"filePath":null}"#)))
        XCTAssertNil(currentPlanPath(nil))
    }
}
