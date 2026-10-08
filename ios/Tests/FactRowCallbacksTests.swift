import XCTest
@testable import Aircast

final class FactRowCallbacksTests: XCTestCase {
    func testATrailingClosurePassedToFactRowRunsAfterAnAcceptedWriteSoCallersReload() {
        let fact = factFromControl(JSON.parse(#"{"name":"N","path":"p","control":"number"}"#))!
        var wrote = false
        let row = FactRow(fact: fact) { wrote = true }
        row.onRejected()
        XCTAssertFalse(wrote)
        row.onWrite()
        XCTAssertTrue(wrote)
    }
}
