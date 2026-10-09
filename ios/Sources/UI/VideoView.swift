import Foundation

let VIDEO_VIEW = "view.video"

struct VideoCamera: Equatable {
    let slot: Int
    let status: String
    let configured: Bool
}

struct SourceSize: Equatable {
    let width: Int
    let height: Int
}

struct VideoReading: Equatable {
    var available: Bool
    var decoding: Bool
    var sourceSize: SourceSize?
    var summary: String
    var activeSource: Int
    var cameras: [VideoCamera]
    var streaming: Bool = false
    var streamEnabled: Bool = true
    var noVideoText: String = ""
    var noVideoReason: String = ""
    var sourceChosen: Bool = true
}

func videoReading(_ view: JSON?) -> VideoReading? {
    guard let view, view["class"].string == "Video" else { return nil }
    let size = view["sourceSize"]
    let width = size["width"].int(0)
    let height = size["height"].int(0)
    return VideoReading(
        available: view["available"].bool,
        decoding: view["decoding"].bool,
        sourceSize: size.object != nil && width > 0 && height > 0 ? SourceSize(width: width, height: height) : nil,
        summary: view["summary"].string,
        activeSource: view["activeSource"].int(0),
        cameras: view["cameras"].objects.map { camera in
            VideoCamera(slot: camera["slot"].int(0), status: camera["status"].string, configured: camera["configured"].bool)
        },
        streaming: view["streaming"].bool,
        streamEnabled: view["streamEnabled"].bool(true),
        noVideoText: view["noVideoText"].string,
        noVideoReason: view["noVideoReason"].string,
        sourceChosen: view["sourceChosen"].bool(true)
    )
}

struct PaintedRect: Equatable {
    let left: Double
    let top: Double
    let width: Double
    let height: Double
}

func paintedRect(_ surfaceWidth: Double, _ surfaceHeight: Double, _ source: SourceSize?) -> PaintedRect {
    guard let source, surfaceWidth > 0, surfaceHeight > 0 else {
        return PaintedRect(left: 0, top: 0, width: surfaceWidth, height: surfaceHeight)
    }
    let scale = min(surfaceWidth / Double(source.width), surfaceHeight / Double(source.height))
    let width = Double(source.width) * scale
    let height = Double(source.height) * scale
    return PaintedRect(left: (surfaceWidth - width) / 2, top: (surfaceHeight - height) / 2, width: width, height: height)
}
