import XCTest
@testable import Aircast

final class QgcTileSourceTests: XCTestCase {
    private func pathFor(_ mapType: String, _ z: Int, _ x: Int, _ y: Int) -> String {
        qgcTileUrl(mapType).removingPrefix("https://\(qgcTileHost)")
            .replacingOccurrences(of: "{z}", with: "\(z)").replacingOccurrences(of: "{x}", with: "\(x)").replacingOccurrences(of: "{y}", with: "\(y)")
    }

    func testATilePathNamesTheMapTypeAndTheTile() {
        XCTAssertEqual(tileAddress(pathFor("Bing Hybrid", 14, 9876, 6543)), TileAddress(mapType: "Bing Hybrid", z: 14, x: 9876, y: 6543))
        XCTAssertEqual(tileAddress(pathFor("Google Street Map", 0, 0, 0)), TileAddress(mapType: "Google Street Map", z: 0, x: 0, y: 0))
    }

    func testAnythingThatIsNotATilePathNamesNoTile() {
        XCTAssertNil(tileAddress("/14/9876/6543"))
        XCTAssertNil(tileAddress("/style.json"))
        XCTAssertNil(tileAddress("/Bing/14/9876/6543.png"))
    }

    func testATileNothingElseServesFallsBackToOpenStreetMap() {
        XCTAssertEqual(osmTileUrl(TileAddress(mapType: "Bing Hybrid", z: 14, x: 9876, y: 6543)), "https://tile.openstreetmap.org/14/9876/6543.png")
        XCTAssertTrue(OSM_RASTER_STYLE.contains(OSM_TILE_URL))
    }

    func testTheMapTypeIsTheProviderAndTypeTheSettingsName() {
        XCTAssertEqual(mapTypeName("Bing", "Hybrid"), "Bing Hybrid")
        XCTAssertEqual(mapTypeName("Bing", ""), "Bing")
        XCTAssertTrue(qgcRasterStyle("Esri World Satellite").contains(qgcTileUrl("Esri World Satellite")))
    }
}
