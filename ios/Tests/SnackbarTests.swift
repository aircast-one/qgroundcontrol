import XCTest
@testable import Aircast

private let YIELDS_BEFORE_GIVING_UP = 1000

@MainActor
private func settle(_ left: Int = YIELDS_BEFORE_GIVING_UP, _ done: () -> Bool) async {
    guard left > 0, !done() else { return }
    await Task.yield()
    await settle(left - 1, done)
}

@MainActor
private func shown(_ host: SnackbarHostState, _ message: String, file: StaticString = #filePath, line: UInt = #line) async throws -> SnackbarData {
    await settle { host.currentSnackbarData?.message == message }
    return try XCTUnwrap(host.currentSnackbarData.flatMap { $0.message == message ? $0 : nil }, "\(message) never showed", file: file, line: line)
}

@MainActor
final class SnackbarTests: XCTestCase {
    func testSnackbarsShowOneAtATimeInTheOrderTheyWereAskedFor() async throws {
        let host = SnackbarHostState()
        let first = Task { await host.showSnackbar("first", duration: .Indefinite) }
        let firstShown = try await shown(host, "first")
        let second = Task { await host.showSnackbar("second", duration: .Indefinite) }
        await settle { false }
        XCTAssertEqual(host.currentSnackbarData?.message, "first", "the second waits until the first is gone")
        firstShown.dismiss()
        let secondShown = try await shown(host, "second")
        let firstResult = await first.value
        XCTAssertEqual(firstResult, .Dismissed)
        secondShown.performAction()
        let secondResult = await second.value
        XCTAssertEqual(secondResult, .ActionPerformed)
        XCTAssertNil(host.currentSnackbarData)
    }

    func testAStaleDismissOfAnEarlierSnackbarLeavesTheNextOneShowing() async throws {
        let host = SnackbarHostState()
        let first = Task { await host.showSnackbar("first", actionLabel: "Undo") }
        let firstShown = try await shown(host, "first")
        let second = Task { await host.showSnackbar("second", actionLabel: "Undo") }
        firstShown.performAction()
        let secondShown = try await shown(host, "second")
        firstShown.dismiss()
        await settle { false }
        XCTAssertTrue(host.currentSnackbarData === secondShown)
        let firstResult = await first.value
        XCTAssertEqual(firstResult, .ActionPerformed, "a snackbar resolves once")
        secondShown.dismiss()
        _ = await second.value
    }

    func testCancellingTheCallerShowingASnackbarHandsTheHostToTheNextOne() async throws {
        let host = SnackbarHostState()
        let stuck = Task { await host.showSnackbar("stuck", duration: .Indefinite) }
        _ = try await shown(host, "stuck")
        let next = Task { await host.showSnackbar("next", duration: .Indefinite) }
        stuck.cancel()
        let stuckResult = await stuck.value
        XCTAssertEqual(stuckResult, .Dismissed)
        let nextShown = try await shown(host, "next")
        nextShown.dismiss()
        _ = await next.value
    }

    func testCancellingAQueuedCallerDropsItWithoutHoldingTheHost() async throws {
        let host = SnackbarHostState()
        let current = Task { await host.showSnackbar("current", duration: .Indefinite) }
        let currentShown = try await shown(host, "current")
        let queued = Task { await host.showSnackbar("queued", duration: .Indefinite) }
        await settle { false }
        queued.cancel()
        let queuedResult = await queued.value
        XCTAssertEqual(queuedResult, .Dismissed)
        currentShown.dismiss()
        _ = await current.value
        let later = Task { await host.showSnackbar("later", duration: .Indefinite) }
        let laterShown = try await shown(host, "later")
        laterShown.dismiss()
        _ = await later.value
    }
}
