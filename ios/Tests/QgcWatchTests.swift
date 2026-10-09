import XCTest
@testable import Aircast

@MainActor
final class QgcWatchTests: XCTestCase {
    private var sent: [String] = []
    private var held: [String] = []
    private var realSend: ((String) -> Void)?

    override func setUp() {
        super.setUp()
        realSend = QgcWatch.sendWatch
        QgcWatch.sendWatch = { [weak self] csv in self?.sent.append(csv) }
    }

    override func tearDown() {
        held.forEach(QgcWatch.release)
        held = []
        realSend.map { QgcWatch.sendWatch = $0 }
        super.tearDown()
    }

    @discardableResult
    private func retain(_ path: String) -> PathSubject {
        held.append(path)
        return QgcWatch.retain(path)
    }

    private func release(_ path: String) {
        if let at = held.firstIndex(of: path) { held.remove(at: at) }
        QgcWatch.release(path)
    }

    func testAMapLeavingTheScreenKeepsThePathsAnotherMapStillWatches() {
        retain("view.plan")
        retain("view.missionKinds")
        retain("view.plan")
        release("view.plan")
        XCTAssertTrue(QgcWatch.watched.isSuperset(of: ["view.plan", "view.missionKinds"]))
    }

    func testTheLastHolderOfAPathUnwatchesIt() {
        retain("view.missionSummary")
        release("view.missionSummary")
        XCTAssertFalse(QgcWatch.watched.contains("view.missionSummary"))
        XCTAssertEqual(sent.last.map { $0.split(separator: ",").contains("view.missionSummary") }, false)
    }

    func testASecondHolderOfAPathSendsNothingNew() {
        retain("view.missionSummary")
        retain("view.missionSummary")
        XCTAssertEqual(sent.count, 1)
        XCTAssertEqual(sent.first.map { $0.split(separator: ",").contains("view.missionSummary") }, true)
    }

    func testAnUnwatchedPathDropsItsSubjectSoOneOffPathsDoNotPileUp() {
        let first = retain("view.track(7)")
        QgcWatch.seed("view.track(7)", .object(["points": .array([])]))
        release("view.track(7)")
        let again = retain("view.track(7)")
        XCTAssertFalse(first === again)
        XCTAssertNil(again.json)
    }
}
