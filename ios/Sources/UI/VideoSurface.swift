import QGCCore
import SwiftUI
import UIKit

let CAMERA_STEP_ZOOM = "vehicle.cameraManager.currentCameraInstance.stepZoom"

func pinchStep(_ scale: CGFloat) -> Int {
    Int(((scale < 1 ? scale * -10 : scale) + 0.5).rounded(.down))
}

private struct PinchZoom: ViewModifier {
    let enabled: Bool
    @State private var sent = 0

    func body(content: Content) -> some View {
        if enabled {
            content.simultaneousGesture(
                MagnifyGesture()
                    .onChanged { pinch in
                        let step = pinchStep(pinch.magnification)
                        if step != sent { offMain { Qgc.invoke(CAMERA_STEP_ZOOM, step) } }
                        sent = step
                    }
                    .onEnded { _ in sent = 0 }
            )
        } else {
            content
        }
    }
}

private struct NoVideoArea: View {
    let underFlyChrome: Bool
    let thumb: Bool
    let video: VideoReading?

    var body: some View {
        if !underFlyChrome {
            NoVideoPanel(video: video)
                .environment(\.noVideoSize, thumb ? .Thumb : .Full)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

struct FlyNoVideoMessage: View {
    @HasVehicle private var hasVehicle
    @QgcPath(VIDEO_VIEW) private var videoJson

    var body: some View {
        if !hasVehicle {
            LookingForAircraft()
        } else {
            let video = videoReading(videoJson)
            GeometryReader { geometry in
                let compact = geometry.size.height < NO_VIDEO_FULL_HEIGHT || geometry.size.width < NO_VIDEO_FULL_WIDTH
                NoVideoPanel(video: video)
                    .environment(\.noVideoSize, compact ? .Pill : .Full)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: compact ? .topTrailing : .center)
            }
        }
    }
}

private func rectFrom(_ left: CGFloat, _ top: CGFloat, _ right: CGFloat, _ bottom: CGFloat) -> CGRect {
    CGRect(x: left, y: top, width: right - left, height: bottom - top)
}

private func rightOf(_ rect: CGRect) -> CGFloat { rect.origin.x + rect.size.width }

private func bottomOf(_ rect: CGRect) -> CGFloat { rect.origin.y + rect.size.height }

private func hasNoArea(_ rect: CGRect) -> Bool { rect.size.width <= 0 || rect.size.height <= 0 }

private func overlapping(_ rect: CGRect, _ other: CGRect) -> Bool {
    rightOf(rect) > other.origin.x && rightOf(other) > rect.origin.x && bottomOf(rect) > other.origin.y && bottomOf(other) > rect.origin.y
}

private func contains(_ rect: CGRect, _ other: CGRect) -> Bool {
    rect.origin.x <= other.origin.x && rect.origin.y <= other.origin.y && rightOf(rect) >= rightOf(other) && bottomOf(rect) >= bottomOf(other)
}

func centredRegion(_ region: CGRect, _ centreX: CGFloat, _ minWidth: CGFloat) -> CGRect {
    let half = min(centreX - region.origin.x, rightOf(region) - centreX)
    return half * 2 >= minWidth ? rectFrom(centreX - half, region.origin.y, centreX + half, bottomOf(region)) : region
}

func messageRegion(_ free: CGRect, _ obstacles: [CGRect], _ gap: CGFloat, _ minWidth: CGFloat, _ minHeight: CGFloat) -> CGRect? {
    let fits = { (room: CGRect) in room.size.width >= minWidth && room.size.height >= minHeight }
    return obstacles.filter { !hasNoArea($0) }
        .reduce([free].filter(fits)) { rooms, obstacle in
            let found = rooms.flatMap { room in overlapping(room, obstacle) ? sidesAround(room, obstacle, gap).filter(fits) : [room] }
            let distinct = found.enumerated().filter { at, room in !found[..<at].contains(room) }.map(\.element)
            return distinct.filter { room in !distinct.contains { other in other != room && contains(other, room) } }
        }
        .max { $0.size.width * $0.size.height < $1.size.width * $1.size.height }
}

private func sidesAround(_ room: CGRect, _ obstacle: CGRect, _ gap: CGFloat) -> [CGRect] {
    [
        rectFrom(rightOf(obstacle) + gap, room.origin.y, rightOf(room), bottomOf(room)),
        rectFrom(room.origin.x, room.origin.y, obstacle.origin.x - gap, bottomOf(room)),
        rectFrom(room.origin.x, bottomOf(obstacle) + gap, rightOf(room), bottomOf(room)),
        rectFrom(room.origin.x, room.origin.y, rightOf(room), obstacle.origin.y - gap),
    ]
}

struct VideoSurface: View {
    var expanded: Bool = false
    var fullScreen: Bool = false
    @Environment(\.theme) private var theme
    @QgcPath(VIDEO_VIEW) private var videoJson
    @QgcPath(CAMERA_VIEW) private var cameraJson
    @QgcBool(settingControl("settings.videoSettings.gridLines")) private var showGrid
    @QgcValue(settingControl("settings.videoSettings.videoFit")) private var fitMode
    @QgcValue(settingControl("settings.videoSettings.aspectRatio")) private var aspectSetting

    var body: some View {
        let video = videoReading(videoJson)
        if video?.available == false {
            if expanded {
                NoVideoArea(underFlyChrome: expanded && !fullScreen, thumb: false, video: video)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(theme.colors.surfaceVariant)
            }
        } else {
            playing(video)
        }
    }

    private func playing(_ video: VideoReading?) -> some View {
        let decoding = video?.decoding == true
        let aspect = videoAspect(video?.sourceSize, aspectSetting.double)
        return GeometryReader { geometry in
            let shown = videoContentSize(geometry.size.width, geometry.size.height, aspect, fitMode.int ?? VIDEO_FIT_HEIGHT)
            ZStack {
                ZStack {
                    VideoChannelSurface(channel: MAIN_VIDEO_CHANNEL)
                    if decoding {
                        if showGrid && !fullScreen { VideoGrid() }
                        ProximityRadarOverlay()
                        ObstacleVideoOverlay(showText: expanded)
                        if expanded { GimbalScreenControl() }
                        DetectionOverlay()
                        TrackingBoxOverlay()
                        if expanded { VideoStatsPill() }
                    }
                }
                .frame(width: shown.0, height: shown.1)
                if !decoding {
                    NoVideoArea(underFlyChrome: expanded && !fullScreen, thumb: !expanded, video: video)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .background(expanded ? theme.colors.surfaceVariant : theme.aircast.outdoorBackground)
                }
            }
            .frame(width: geometry.size.width, height: geometry.size.height)
        }
        .clipped()
        .contentShape(Rectangle())
        .modifier(PinchZoom(enabled: expanded && cameraReading(cameraJson)?.hasZoom == true))
    }
}

let MAIN_VIDEO_CHANNEL = 0
let PIP_VIDEO_CHANNEL = 1

final class PipSurfaces {
    private let report: (Bool) -> Void
    private let lock = NSLock()
    private var attached = 0

    init(_ report: @escaping (Bool) -> Void) {
        self.report = report
    }

    func created() { report(changed(by: 1) > 0) }

    func destroyed() { report(changed(by: -1) > 0) }

    private func changed(by step: Int) -> Int {
        lock.withLock {
            attached += step
            return attached
        }
    }
}

private let pipSurfaces = PipSurfaces { shown in offMainInOrder { VideoCommands.setPipShown(shown) } }

private struct DecodedFrame: Equatable {
    let width: Int
    let height: Int
    let stride: Int

    init?(width: Int32, height: Int32, stride: Int32) {
        guard width > 0, height > 0, stride > 0, Int(stride) >= Int(width) * DecodedFrame.bytesPerPixel else { return nil }
        self.width = Int(width)
        self.height = Int(height)
        self.stride = Int(stride)
    }

    static let bytesPerPixel = 4
    static let rowAlignment = 64

    var byteCount: Int { stride * height }

    static func capacity(width: Int, height: Int) -> Int {
        guard width > 0, height > 0 else { return 0 }
        let row = width * bytesPerPixel
        return (row + rowAlignment - 1) / rowAlignment * rowAlignment * height
    }

    func fits(_ available: Int) -> Bool { available >= byteCount }

    func image(_ bytes: Data) -> CGImage? {
        guard fits(bytes.count), let provider = CGDataProvider(data: bytes as CFData) else { return nil }
        return CGImage(
            width: width,
            height: height,
            bitsPerComponent: 8,
            bitsPerPixel: 32,
            bytesPerRow: stride,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue),
            provider: provider,
            decode: nil,
            shouldInterpolate: true,
            intent: .defaultIntent
        )
    }
}

private final class FrameTicker: NSObject {
    weak var view: VideoChannelLayerView?

    init(_ view: VideoChannelLayerView) {
        self.view = view
    }

    @objc func tick() { view?.drawFrame() }
}

final class VideoChannelLayerView: UIView {
    private let channel: Int32
    private var link: CADisplayLink?
    private var pipReported = false
    private var copying = false
    private var shownFrames: Int64 = -1

    init(channel: Int) {
        self.channel = Int32(channel)
        super.init(frame: .zero)
        backgroundColor = .black
        isUserInteractionEnabled = false
        layer.contentsGravity = .resizeAspect
    }

    required init?(coder: NSCoder) { nil }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        let onScreen = window != nil
        if onScreen { start() } else { stop() }
        guard channel == Int32(PIP_VIDEO_CHANNEL), onScreen != pipReported else { return }
        pipReported = onScreen
        if onScreen { pipSurfaces.created() } else { pipSurfaces.destroyed() }
    }

    private func start() {
        guard link == nil else { return }
        let made = CADisplayLink(target: FrameTicker(self), selector: #selector(FrameTicker.tick))
        made.preferredFrameRateRange = CAFrameRateRange(minimum: 10, maximum: 30, preferred: 30)
        made.add(to: .main, forMode: .common)
        link = made
    }

    func stop() {
        link?.invalidate()
        link = nil
    }

    fileprivate func drawFrame() {
        guard !copying else { return }
        copying = true
        let channel = self.channel
        let shown = shownFrames
        frameCopies.async { [weak self] in
            let frames = qgc_video_frames(channel)
            let image = frames == shown ? nil : copiedFrame(channel)
            DispatchQueue.main.async {
                guard let self else { return }
                self.copying = false
                guard let image else { return }
                self.shownFrames = frames
                self.layer.contents = image
            }
        }
    }
}

private let frameCopies = DispatchQueue(label: "one.aircast.video-frames", qos: .userInteractive)

private func copiedFrame(_ channel: Int32) -> CGImage? {
    let capacity = DecodedFrame.capacity(width: Int(qgc_video_width(channel)), height: Int(qgc_video_height(channel)))
    guard capacity > 0 else { return nil }
    var copiedWidth: Int32 = 0
    var copiedHeight: Int32 = 0
    var stride: Int32 = 0
    var bytes = Data(count: capacity)
    let copied = bytes.withUnsafeMutableBytes { destination in
        destination.baseAddress.map { qgc_video_copy_frame(channel, $0, Int32(destination.count), &copiedWidth, &copiedHeight, &stride) } ?? false
    }
    guard copied, let frame = DecodedFrame(width: copiedWidth, height: copiedHeight, stride: stride) else { return nil }
    return frame.image(bytes)
}

struct VideoChannelSurface: UIViewRepresentable {
    let channel: Int

    func makeUIView(context: Context) -> VideoChannelLayerView { VideoChannelLayerView(channel: channel) }

    func updateUIView(_ uiView: VideoChannelLayerView, context: Context) {}

    static func dismantleUIView(_ uiView: VideoChannelLayerView, coordinator: ()) { uiView.stop() }
}

private let NO_VIDEO_FULL_HEIGHT: CGFloat = 230
private let NO_VIDEO_FULL_WIDTH: CGFloat = 260
let NO_VIDEO_PILL_WIDTH: CGFloat = 180
let NO_VIDEO_PILL_HEIGHT: CGFloat = 48
let VIDEO_FIT_WIDTH = 0
let VIDEO_FIT_HEIGHT = 1
let VIDEO_FILL = 2
let VIDEO_NO_CROP = 3
private let GRID_FRACTIONS = 3
private let GRID_COLOUR = Color.white.opacity(0.5)

func videoAspect(_ source: SourceSize?, _ setting: Double?) -> Double {
    source.flatMap { $0.width > 0 && $0.height > 0 ? Double($0.width) / Double($0.height) : nil } ?? setting.flatMap { $0 > 0 ? $0 : nil } ?? 0
}

func videoContentSize(_ width: CGFloat, _ height: CGFloat, _ aspect: Double, _ fit: Int) -> (CGFloat, CGFloat) {
    if aspect == 0 || width <= 0 || height <= 0 { return (width, height) }
    let box = Double(width / height)
    let fitsHeight = fit == VIDEO_FIT_HEIGHT || (fit == VIDEO_FILL && box < aspect) || (fit == VIDEO_NO_CROP && box > aspect)
    let fitsWidth = fit == VIDEO_FIT_WIDTH || (fit == VIDEO_FILL && box > aspect) || (fit == VIDEO_NO_CROP && box < aspect)
    return (fitsHeight ? height * CGFloat(aspect) : width, fitsWidth ? width / CGFloat(aspect) : height)
}

private struct VideoGrid: View {
    var body: some View {
        Canvas { context, size in
            (1..<GRID_FRACTIONS).forEach { line in
                let x = size.width * CGFloat(line) * 0.33
                let y = size.height * CGFloat(line) * 0.33
                context.stroke(Path { $0.move(to: CGPoint(x: x, y: 0)); $0.addLine(to: CGPoint(x: x, y: size.height)) }, with: .color(GRID_COLOUR), lineWidth: 1)
                context.stroke(Path { $0.move(to: CGPoint(x: 0, y: y)); $0.addLine(to: CGPoint(x: size.width, y: y)) }, with: .color(GRID_COLOUR), lineWidth: 1)
            }
        }
        .allowsHitTesting(false)
    }
}
