import XCTest
@testable import Aircast

final class QgcTileSourceTests: XCTestCase {
    private func pathFor(_ mapType: String, _ z: Int, _ x: Int, _ y: Int) -> String {
        qgcTileUrl(mapType).removingPrefix("https://\(QGC_TILE_HOST)")
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

    func testATilePathIsDecodedOnceSoAPercentInTheMapTypeSurvives() throws {
        let url = try XCTUnwrap(URL(string: "https://\(QGC_TILE_HOST)\(pathFor("50% Grey", 3, 4, 5))"))
        XCTAssertEqual(tileAddress(url), TileAddress(mapType: "50% Grey", z: 3, x: 4, y: 5))
    }

    func testSignedOrOversizedTileNumbersNameNoTile() {
        XCTAssertNil(tileAddress("/Bing/-1/2/3"))
        XCTAssertNil(tileAddress("/Bing/+5/2/3"))
        XCTAssertNil(tileAddress("/Bing/3/99999999999/3"))
        XCTAssertNil(tileAddress("//Bing/3/2/3"))
    }

    func testTheStyleFileHoldsTheRasterStyle() throws {
        let url = try XCTUnwrap(qgcStyleURL("Bing Hybrid"))
        XCTAssertEqual(try String(contentsOf: url, encoding: .utf8), qgcRasterStyle("Bing Hybrid"))
    }
}

private final class RecordingClient: NSObject, URLProtocolClient {
    private let lock = NSLock()
    private var recorded: [(String, Bool)] = []
    var finished: XCTestExpectation?

    var events: [(String, Bool)] { lock.withLock { recorded } }

    private func record(_ event: String) {
        lock.withLock { recorded.append((event, Thread.isMainThread)) }
        if event == "finish" { finished?.fulfill() }
    }

    func urlProtocol(_ p: URLProtocol, wasRedirectedTo request: URLRequest, redirectResponse: URLResponse) { record("redirect") }
    func urlProtocol(_ p: URLProtocol, cachedResponseIsValid cachedResponse: CachedURLResponse) { record("cached") }
    func urlProtocol(_ p: URLProtocol, didReceive response: URLResponse, cacheStoragePolicy policy: URLCache.StoragePolicy) { record("response") }
    func urlProtocol(_ p: URLProtocol, didLoad data: Data) { record("data") }
    func urlProtocolDidFinishLoading(_ p: URLProtocol) { record("finish") }
    func urlProtocol(_ p: URLProtocol, didFailWithError error: Error) { record("error") }
    func urlProtocol(_ p: URLProtocol, didReceive challenge: URLAuthenticationChallenge) { record("challenge") }
    func urlProtocol(_ p: URLProtocol, didCancel challenge: URLAuthenticationChallenge) { record("cancel") }
}

final class QgcTileProtocolTests: XCTestCase {
    private let tile = Data([0x89, 0x50, 0x4E, 0x47, 0x00])
    private let issued: UInt64 = 42
    private var reply: ((Data?) -> Void)?
    private var cancelled: [UInt64] = []
    private var coreFetch: ((TileAddress, @escaping (Data?) -> Void) -> UInt64)?
    private var coreCancel: ((UInt64) -> Void)?

    override func setUp() {
        coreFetch = MapTileHost.fetch
        coreCancel = MapTileHost.cancel
        MapTileHost.fetch = { [weak self, issued] _, reply in self?.reply = reply; return issued }
        MapTileHost.cancel = { [weak self] ticket in self?.cancelled.append(ticket) }
    }

    override func tearDown() {
        coreFetch.map { MapTileHost.fetch = $0 }
        coreCancel.map { MapTileHost.cancel = $0 }
    }

    private func started(_ client: RecordingClient) throws -> QgcTileProtocol {
        let url = try XCTUnwrap(URL(string: qgcTileUrl("Bing Hybrid").replacingOccurrences(of: "{z}", with: "3")
            .replacingOccurrences(of: "{x}", with: "4").replacingOccurrences(of: "{y}", with: "5")))
        let loading = QgcTileProtocol(request: URLRequest(url: url), cachedResponse: nil, client: client)
        loading.startLoading()
        return loading
    }

    func testACoreReplyFromItsWorkerReachesTheClientOnTheLoadingThread() throws {
        let client = RecordingClient()
        client.finished = expectation(description: "finished")
        let loading = try started(client)
        let answer = try XCTUnwrap(reply)
        let tile = tile
        DispatchQueue.global().async { answer(tile) }
        wait(for: [try XCTUnwrap(client.finished)], timeout: 5)
        XCTAssertEqual(client.events.map(\.0), ["response", "data", "finish"])
        XCTAssertTrue(client.events.allSatisfy(\.1))
        withExtendedLifetime(loading) {}
    }

    func testACoreReplyAfterTheLoadStoppedNeverReachesTheClient() throws {
        let client = RecordingClient()
        let loading = try started(client)
        loading.stopLoading()
        let answer = try XCTUnwrap(reply)
        let drained = expectation(description: "drained")
        let tile = tile
        DispatchQueue.global().async {
            answer(tile)
            CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue) { drained.fulfill() }
            CFRunLoopWakeUp(CFRunLoopGetMain())
        }
        wait(for: [drained], timeout: 5)
        XCTAssertTrue(client.events.isEmpty)
        withExtendedLifetime(loading) {}
    }

    func testStoppingALoadCancelsTheCoreTileItStarted() throws {
        let loading = try started(RecordingClient())
        XCTAssertTrue(cancelled.isEmpty)
        loading.stopLoading()
        XCTAssertEqual(cancelled, [issued])
    }
}
