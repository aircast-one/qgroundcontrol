import AppKit
import Foundation
import MapKit
import QGCMapTileC

final class CachedTileOverlay: MKTileOverlay {
    private let mapType: String

    init(mapType: String) {
        self.mapType = mapType
        super.init(urlTemplate: nil)
        tileSize = CGSize(width: 256, height: 256)
        canReplaceMapContent = true
    }

    static func currentMapType() -> String {
        guard let raw = qgc_map_current_type() else { return "" }
        defer { free(raw) }
        return String(cString: raw)
    }

    static var lastRequest: Date?
    static var requested = 0
    static var served = 0
    static var fromParent = 0
    static var fromChildren = 0
    static var missed = 0

    override func loadTile(at path: MKTileOverlayPath,
                           result: @escaping (Data?, Error?) -> Void) {
        CachedTileOverlay.requested += 1
        CachedTileOverlay.lastRequest = Date()
        let tile = TileAddress(x: path.x, y: path.y, z: path.z)
        fetch(tile) { data in
            if let data {
                CachedTileOverlay.served += 1
                return result(data, nil)
            }
            self.fromParent(of: tile, depth: 1) { data in
                if let data {
                    CachedTileOverlay.fromParent += 1
                    return result(data, nil)
                }
                self.fromChildren(of: tile) { data in
                    data == nil ? (CachedTileOverlay.missed += 1) : (CachedTileOverlay.fromChildren += 1)
                    result(data, nil)
                }
            }
        }
    }

    private func fromParent(of tile: TileAddress, depth: Int,
                            completion: @escaping (Data?) -> Void) {
        guard depth <= TilePyramid.maximumParentDepth,
              let parent = TilePyramid.parent(of: tile, depth: depth),
              let crop = TilePyramid.crop(of: tile, within: parent) else {
            return completion(nil)
        }

        fetch(parent) { data in
            guard let data, let image = CachedTileOverlay.image(from: data) else {
                return self.fromParent(of: tile, depth: depth + 1, completion: completion)
            }
            completion(CachedTileOverlay.scaled(image, crop: crop))
        }
    }

    private func fromChildren(of tile: TileAddress, completion: @escaping (Data?) -> Void) {
        let children = TilePyramid.children(of: tile)
        var found = [Int: CGImage]()
        var outstanding = children.count

        children.enumerated().forEach { index, child in
            fetch(child) { data in
                if let data, let image = CachedTileOverlay.image(from: data) {
                    found[index] = image
                }
                outstanding -= 1
                guard outstanding == 0 else { return }
                completion(found.isEmpty ? nil : CachedTileOverlay.composed(found, of: children))
            }
        }
    }

    private func fetch(_ tile: TileAddress, completion: @escaping (Data?) -> Void) {
        let box = Unmanaged.passRetained(TileRequest(completion: completion)).toOpaque()

        qgc_map_tile_fetch(mapType, Int32(tile.x), Int32(tile.y), Int32(tile.z), { bytes, length, context in
            guard let context else { return }
            let request = Unmanaged<TileRequest>.fromOpaque(context).takeRetainedValue()
            guard let bytes, length > 0 else { return request.completion(nil) }
            request.completion(Data(bytes: bytes, count: Int(length)))
        }, box)
    }

    private static func image(from data: Data) -> CGImage? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
        return CGImageSourceCreateImageAtIndex(source, 0, nil)
    }

    private static func encoded(_ context: CGContext) -> Data? {
        guard let image = context.makeImage() else { return nil }
        return NSBitmapImageRep(cgImage: image).representation(
            using: .jpeg, properties: [.compressionFactor: 0.85])
    }

    private static func context() -> CGContext? {
        CGContext(data: nil, width: 256, height: 256, bitsPerComponent: 8, bytesPerRow: 0,
                  space: CGColorSpaceCreateDeviceRGB(),
                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    }

    private static func scaled(_ image: CGImage, crop: TileCrop) -> Data? {
        let width = Double(image.width)
        let height = Double(image.height)
        let rect = CGRect(x: crop.x * width, y: crop.y * height,
                          width: crop.size * width, height: crop.size * height)

        guard let cropped = image.cropping(to: rect), let context = context() else { return nil }
        context.interpolationQuality = .high
        context.draw(cropped, in: CGRect(x: 0, y: 0, width: 256, height: 256))
        return encoded(context)
    }

    private static func composed(_ found: [Int: CGImage], of children: [TileAddress]) -> Data? {
        guard let context = context() else { return nil }
        context.interpolationQuality = .high

        found.forEach { index, image in
            let quadrant = TilePyramid.quadrant(of: children[index])
            context.draw(image, in: CGRect(x: CGFloat(quadrant.column) * 128,
                                           y: CGFloat(1 - quadrant.row) * 128,
                                           width: 128, height: 128))
        }
        return encoded(context)
    }
}

private final class TileRequest {
    let completion: (Data?) -> Void
    init(completion: @escaping (Data?) -> Void) { self.completion = completion }
}
