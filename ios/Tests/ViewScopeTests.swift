import XCTest
@testable import Aircast

@MainActor
final class ViewScopeTests: XCTestCase {
    func testCancelReachesWorkStillRunning() async {
        let scope = ViewScope()
        let cancelled = expectation(description: "cancelled")
        scope.launch {
            _ = await AsyncStream<Int> { _ in }.first { _ in true }
            if Task.isCancelled { cancelled.fulfill() }
        }
        scope.cancel()
        await fulfillment(of: [cancelled], timeout: 2)
    }

    func testWorkLaunchedAfterCancelStillRuns() async {
        let scope = ViewScope()
        scope.cancel()
        let ran = expectation(description: "ran")
        scope.launch { if !Task.isCancelled { ran.fulfill() } }
        await fulfillment(of: [ran], timeout: 2)
    }
}
