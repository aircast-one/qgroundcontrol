import XCTest
@testable import Aircast

final class TerrainDataWarningTests: XCTestCase {
    func testTheWarningWatchesForBlocksStillPendingAndALoadThatFinishedOrNeverStartedIsQuiet() {
        let loading = terrainLoad(JSON.parse(#"{"loaded":3,"pending":1,"fraction":0.75}"#))
        XCTAssertEqual(loading, TerrainLoad(loaded: 3, pending: 1))
        XCTAssertTrue(loading.waiting)
        XCTAssertFalse(terrainLoad(JSON.parse(#"{"loaded":5,"pending":0}"#)).waiting, "everything loaded is nothing to warn about")
        XCTAssertFalse(terrainLoad(JSON.parse(#"{"kind":"null"}"#)).waiting, "no vehicle, no terrain view")
        XCTAssertFalse(terrainLoad(nil).waiting)
    }
}
