import Foundation
import MapKit
import QGCMapTileC

// QGC already downloads and caches map imagery in SQLite; the native map should draw
// that rather than a second copy from Apple. It is also the only imagery available at
// all when the ground station has no signal, which is the case that matters.
final class CachedTileOverlay: MKTileOverlay {
    private let mapType: String

    init(mapType: String) {
        self.mapType = mapType
        super.init(urlTemplate: nil)
        tileSize = CGSize(width: 256, height: 256)
        // The cache holds imagery, not labels, so it replaces Apple's map rather than
        // drawing over it.
        canReplaceMapContent = true
    }

    static func currentMapType() -> String {
        guard let raw = qgc_map_current_type() else { return "" }
        defer { free(raw) }
        return String(cString: raw)
    }

    override func loadTile(at path: MKTileOverlayPath,
                           result: @escaping (Data?, Error?) -> Void) {
        // The handler is a C function pointer and cannot capture, so the continuation
        // travels as an unmanaged context pointer and is consumed exactly once.
        let box = Unmanaged.passRetained(TileRequest(completion: result)).toOpaque()

        qgc_map_tile_fetch(mapType, Int32(path.x), Int32(path.y), Int32(path.z), { bytes, length, context in
            guard let context else { return }
            let request = Unmanaged<TileRequest>.fromOpaque(context).takeRetainedValue()
            guard let bytes, length > 0 else {
                request.completion(nil, nil)
                return
            }
            request.completion(Data(bytes: bytes, count: Int(length)), nil)
        }, box)
    }
}

private final class TileRequest {
    let completion: (Data?, Error?) -> Void
    init(completion: @escaping (Data?, Error?) -> Void) { self.completion = completion }
}
