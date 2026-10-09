import Foundation
import MapLibre
import QGCCore

let QGC_TILE_HOST = "qgc.tiles"
private let MAP_PROVIDER = "settings.flightMapSettings.mapProvider.rawValue"
private let MAP_TYPE = "settings.flightMapSettings.mapType.rawValue"
private let PNG_MAGIC: [UInt8] = [0x89, 0x50, 0x4E, 0x47]

struct TileAddress: Equatable {
    let mapType: String
    let z: Int
    let x: Int
    let y: Int
}

func tileAddress(_ path: String) -> TileAddress? {
    let parts = path.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
    guard parts.count == 4, let z = Int(parts[1]), let x = Int(parts[2]), let y = Int(parts[3]),
          let type = parts[0].removingPercentEncoding else { return nil }
    return TileAddress(mapType: type, z: z, x: x, y: y)
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

func qgcStyleURL(_ mapType: String) -> URL {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent("map-styles", isDirectory: true)
    try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    let name = mapType.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? "default"
    let file = folder.appendingPathComponent("\(name.ifBlank("default")).json")
    try? qgcRasterStyle(mapType).write(to: file, atomically: true, encoding: .utf8)
    return file
}

final class QgcTileProtocol: URLProtocol {
    private var fallbackTask: URLSessionDataTask?

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
        guard let url = request.url, let address = tileAddress(url.path) else { return fail(404) }
        let box = Unmanaged.passRetained(TileReply { [weak self] data in self?.served(address, data) }).toOpaque()
        qgc_map_tile_fetch(address.mapType, Int32(address.x), Int32(address.y), Int32(address.z), { bytes, length, context in
            guard let context else { return }
            let reply = Unmanaged<TileReply>.fromOpaque(context).takeRetainedValue()
            reply.done(bytes.flatMap { length > 0 ? Data(bytes: $0, count: Int(length)) : nil })
        }, box)
    }

    override func stopLoading() {
        fallbackTask?.cancel()
    }

    private func served(_ address: TileAddress, _ data: Data?) {
        if let data { return respond(data) }
        guard let fallback = URL(string: osmTileUrl(address)) else { return fail(404) }
        var request = URLRequest(url: fallback)
        request.setValue("Aircast/iOS", forHTTPHeaderField: "User-Agent")
        fallbackTask = URLSession.shared.dataTask(with: request) { [weak self] data, response, _ in
            guard let data, (response as? HTTPURLResponse)?.statusCode == 200 else { return self?.fail(404) ?? () }
            self?.respond(data)
        }
        fallbackTask?.resume()
    }

    private func respond(_ data: Data) {
        guard let url = request.url else { return }
        let png = data.prefix(PNG_MAGIC.count).elementsEqual(PNG_MAGIC)
        let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1",
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

private final class TileReply {
    let done: (Data?) -> Void
    init(done: @escaping (Data?) -> Void) { self.done = done }
}
