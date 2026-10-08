import XCTest
@testable import Aircast

final class TerrainProgressTests: XCTestCase {
    func testProgressShowsWhileBlocksArePendingOrOnceAnyHaveLoaded() throws {
        let loading = try XCTUnwrap(terrainLoad(JSON.parse(#"{"loaded":3,"pending":1,"fraction":0.75}"#)))
        XCTAssertEqual(loading.fraction, 0.75)
        XCTAssertTrue(terrainShowsNow(loading))
        XCTAssertTrue(terrainShowsNow(TerrainLoad(loaded: 5, pending: 0, fraction: 1)), "a load that starts out complete still shows, then hides")
        XCTAssertFalse(terrainShowsNow(TerrainLoad(loaded: 0, pending: 0, fraction: 0)))
        XCTAssertNil(terrainLoad(JSON.parse(#"{"kind":"null"}"#)))
    }
}
