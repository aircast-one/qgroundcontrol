import Foundation
import MapLibre
import QGCCore

let QGC_TILE_HOST = "qgc.tiles"
private let MAP_PROVIDER = "settings.flightMapSettings.mapProvider.rawValue"
private let MAP_TYPE = "settings.flightMapSettings.mapType.rawValue"
private let PNG_MAGIC: [UInt8] = [0x89, 0x50, 0x4E, 0x47]
private let TILE_PATH = #/^/([^/]+)/([0-9]+)/([0-9]+)/([0-9]+)$/#
private let HTTP_OK = 200
private let HTTP_NOT_FOUND = 404
private let TILE_USER_AGENT = "Aircast/iOS"

struct TileAddress: Equatable {
    let mapType: String
    let z: Int
    let x: Int
    let y: Int
}

func tileAddress(_ path: String) -> TileAddress? {
    guard let match = path.wholeMatch(of: TILE_PATH), let type = String(match.1).removingPercentEncoding,
          let z = Int32(match.2), let x = Int32(match.3), let y = Int32(match.4) else { return nil }
    return TileAddress(mapType: type, z: Int(z), x: Int(x), y: Int(y))
}

func tileAddress(_ url: URL) -> TileAddress? {
    URLComponents(url: url, resolvingAgainstBaseURL: false).flatMap { tileAddress($0.percentEncodedPath) }
}

func qgcTileUrl(_ mapType: String) -> String {
    let encoded = mapType.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? mapType
    return "https://\(QGC_TILE_HOST)/\(encoded)/{z}/{x}/{y}"
}

func osmTileUrl(_ address: TileAddress) -> String {
    OSM_TILE_URL.replacingOccurrences(of: "{z}", with: "\(address.z)")
        .replacingOccurrences(of: "{x}", with: "\(address.x)")
        .replacingOccurrences(of: "{y}", with: "\(address.y)")
}

func mapTypeName(_ provider: String, _ type: String) -> String {
    [provider, type].filter { !$0.isBlank }.joined(separator: " ")
}

func currentMapType() -> String {
    mapTypeName(Qgc.get(MAP_PROVIDER)["value"].string, Qgc.get(MAP_TYPE)["value"].string)
}

func qgcRasterStyle(_ mapType: String) -> String {
    """
    {
      "version": 8,
      "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
      "sources": {
        "qgc": {
          "type": "raster",
          "tiles": ["\(qgcTileUrl(mapType))"],
          "tileSize": 256,
          "maxzoom": 20,
          "attribution": "\(mapType)"
        }
      },
      "layers": [ { "id": "qgc", "type": "raster", "source": "qgc" } ]
    }
    """
}

private let STYLE_FOLDER = "map-styles"

func writtenStyleURL(_ json: String, _ name: String) -> URL? {
    let files = FileManager.default
    let folder = files.temporaryDirectory.appendingPathComponent(STYLE_FOLDER, isDirectory: true)
    let file = folder.appendingPathComponent("\(name).json")
    let written = (try? files.createDirectory(at: folder, withIntermediateDirectories: true)) != nil
        && (try? json.write(to: file, atomically: true, encoding: .utf8)) != nil
    return written || files.fileExists(atPath: file.path) ? file : nil
}

func qgcStyleURL(_ mapType: String) -> URL? {
    let name = mapType.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? "default"
    return writtenStyleURL(qgcRasterStyle(mapType), name.ifBlank("default"))
}

private final class TileReply {
    let done: (Data?) -> Void
    init(done: @escaping (Data?) -> Void) { self.done = done }
}

private func coreTileFetch(_ address: TileAddress, _ reply: @escaping (Data?) -> Void) -> UInt64 {
    let box = Unmanaged.passRetained(TileReply(done: reply)).toOpaque()
    return qgc_map_tile_fetch_cancellable(address.mapType, Int32(clamping: address.x), Int32(clamping: address.y), Int32(clamping: address.z), { bytes, length, context in
        guard let context else { return }
        let reply = Unmanaged<TileReply>.fromOpaque(context).takeRetainedValue()
        reply.done(bytes.flatMap { length > 0 ? Data(bytes: $0, count: Int(length)) : nil })
    }, box)
}

enum MapTileHost {
    static var fetch: (TileAddress, @escaping (Data?) -> Void) -> UInt64 = coreTileFetch
    static var cancel: (UInt64) -> Void = qgc_map_tile_cancel
}

final class QgcTileProtocol: URLProtocol {
    private var loader: CFRunLoop?
    private var loaderModes: [RunLoop.Mode] = [.default]
    private var stopped = false
    private var fallbackTask: URLSessionDataTask?
    private var ticket: UInt64 = 0

    static func install() {
        let configuration = MLNNetworkConfiguration.sharedManager.sessionConfiguration ?? URLSessionConfiguration.default
        configuration.protocolClasses = [QgcTileProtocol.self] + (configuration.protocolClasses ?? [])
        MLNNetworkConfiguration.sharedManager.sessionConfiguration = configuration
    }

    override class func canInit(with request: URLRequest) -> Bool {
        request.url?.host == QGC_TILE_HOST
    }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        loader = CFRunLoopGetCurrent()
        loaderModes = [.default] + [RunLoop.current.currentMode].compactMap { $0 }.filter { $0 != .default }
        guard let url = request.url, let address = tileAddress(url) else { return fail(HTTP_NOT_FOUND) }
        ticket = MapTileHost.fetch(address) { [weak self] data in self?.onLoader { $0.served(address, data) } }
    }

    override func stopLoading() {
        stopped = true
        MapTileHost.cancel(ticket)
        fallbackTask?.cancel()
    }

    private func onLoader(_ work: @escaping (QgcTileProtocol) -> Void) {
        guard let loader else { return }
        CFRunLoopPerformBlock(loader, loaderModes.map(\.rawValue) as CFArray) { [weak self] in
            guard let self, !self.stopped else { return }
            work(self)
        }
        CFRunLoopWakeUp(loader)
    }

    private func served(_ address: TileAddress, _ data: Data?) {
        if let data { return respond(data) }
        guard let fallback = URL(string: osmTileUrl(address)) else { return fail(HTTP_NOT_FOUND) }
        var request = URLRequest(url: fallback)
        request.setValue(TILE_USER_AGENT, forHTTPHeaderField: "User-Agent")
        let task = URLSession.shared.dataTask(with: request) { [weak self] data, response, _ in
            self?.onLoader { tile in
                guard let data, (response as? HTTPURLResponse)?.statusCode == HTTP_OK else { return tile.fail(HTTP_NOT_FOUND) }
                tile.respond(data)
            }
        }
        fallbackTask = task
        task.resume()
    }

    private func respond(_ data: Data) {
        guard let url = request.url else { return }
        let png = data.prefix(PNG_MAGIC.count).elementsEqual(PNG_MAGIC)
        let response = HTTPURLResponse(url: url, statusCode: HTTP_OK, httpVersion: "HTTP/1.1",
                                       headerFields: ["Content-Type": png ? "image/png" : "image/jpeg"])
        response.map { client?.urlProtocol(self, didReceive: $0, cacheStoragePolicy: .notAllowed) }
        client?.urlProtocol(self, didLoad: data)
        client?.urlProtocolDidFinishLoading(self)
    }

    private func fail(_ code: Int) {
        guard let url = request.url else { return }
        HTTPURLResponse(url: url, statusCode: code, httpVersion: "HTTP/1.1", headerFields: nil)
            .map { client?.urlProtocol(self, didReceive: $0, cacheStoragePolicy: .notAllowed) }
        client?.urlProtocolDidFinishLoading(self)
    }
}
