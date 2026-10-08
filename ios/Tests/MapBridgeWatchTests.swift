import XCTest
@testable import Aircast

@MainActor
final class MapBridgeWatchTests: XCTestCase {
    private var sent: [String] = []
    private var realSend: ((String) -> Void)?

    override func setUp() {
        super.setUp()
        realSend = QgcWatch.sendWatch
        QgcWatch.sendWatch = { [weak self] csv in self?.sent.append(csv) }
    }

    override func tearDown() {
        realSend.map { QgcWatch.sendWatch = $0 }
        super.tearDown()
    }

    func testAMapLeavingTheScreenKeepsThePathsAnotherMapStillWatches() {
        MapBridge.watch("view.plan")
        MapBridge.watch("view.missionKinds")
        MapBridge.watch("view.plan")
        MapBridge.unwatch("view.plan")
        XCTAssertTrue(MapBridge.watchedPaths.isSuperset(of: ["view.plan", "view.missionKinds"]))
        MapBridge.unwatch("view.plan")
        MapBridge.unwatch("view.missionKinds")
    }

    func testTheLastHolderOfAPathUnwatchesIt() {
        MapBridge.watch("view.missionSummary")
        MapBridge.unwatch("view.missionSummary")
        XCTAssertFalse(MapBridge.watchedPaths.contains("view.missionSummary"))
        XCTAssertEqual(sent.last.map { $0.split(separator: ",").contains("view.missionSummary") }, false)
    }

    func testASecondHolderOfAPathSendsNothingNew() {
        MapBridge.watch("view.missionSummary")
        MapBridge.watch("view.missionSummary")
        XCTAssertEqual(sent.count, 1)
        XCTAssertEqual(sent.first.map { $0.split(separator: ",").contains("view.missionSummary") }, true)
        MapBridge.unwatch("view.missionSummary")
        MapBridge.unwatch("view.missionSummary")
    }
}
