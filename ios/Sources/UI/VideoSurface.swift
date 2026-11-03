import QGCCore
import SwiftUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

let CAMERA_STEP_ZOOM = "vehicle.cameraManager.currentCameraInstance.stepZoom"

func pinchStep(_ scale: CGFloat) -> Int {
    Int(((scale < 1 ? scale * -10 : scale) + 0.5).rounded(.down))
}

private struct PinchZoom: ViewModifier {
    let enabled: Bool
    @State private var sent = 0

    func body(content: Content) -> some View {
        content.simultaneousGesture(
            MagnifyGesture()
                .onChanged { pinch in
                    let step = pinchStep(pinch.magnification)
                    if step != sent { offMain { Qgc.invoke(CAMERA_STEP_ZOOM, step) } }
                    sent = step
                }
                .onEnded { _ in sent = 0 },
            including: enabled ? .all : .subviews
        )
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

func looksForAircraft(connected: Bool, video: VideoReading?, syntheticChosen: Bool = false) -> Bool {
    !connected && (video?.available != true || syntheticChosen)
}

struct FlyNoVideoMessage: View {
    @HasVehicle private var hasVehicle
    @QgcPath(VIDEO_VIEW) private var videoJson
    @QgcPath(SYNTHETIC_VIEW) private var syntheticJson
    @ShownCameras private var cameras

    var body: some View {
        let video = videoReading(videoJson)
        let chosen = syntheticChosen(cameras)
        if looksForAircraft(connected: hasVehicle, video: video, syntheticChosen: chosen) {
            AwaitingAircraftLine(syntheticChosen: chosen)
        } else if syntheticAvailable(syntheticJson) {
            EmptyView()
        } else {
            GeometryReader { geometry in
                let compact = geometry.size.height < NO_VIDEO_FULL_HEIGHT || geometry.size.width < NO_VIDEO_FULL_WIDTH
                NoVideoPanel(video: video)
                    .environment(\.noVideoSize, compact ? .Pill : .Full)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
            }
        }
    }
}

func awaitingAircraftText(syntheticChosen: Bool) -> String {
    syntheticChosen ? "Synthetic view starts when your aircraft connects" : "Video starts when your aircraft connects"
}

private let AWAITING_ALPHA = 0.7
private let AWAITING_ICON_SIZE: CGFloat = 15

private struct AwaitingAircraftLine: View {
    let syntheticChosen: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s2) {
            Image(.videocam).font(.system(size: AWAITING_ICON_SIZE))
            Text(awaitingAircraftText(syntheticChosen: syntheticChosen)).font(.bodyMedium).multilineTextAlignment(.center)
        }
        .foregroundStyle(theme.aircast.outdoorForeground.opacity(AWAITING_ALPHA))
        .padding(.horizontal, Space.s4)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
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
    var onGround: ((PicturePress) -> Void)? = nil
    var onPictureTap: () -> Void = {}
    var groundSpot: CGPoint? = nil
    @Environment(\.theme) private var theme
    @QgcPath(VIDEO_VIEW) private var videoJson
    @QgcPath(GIMBAL_INDICATOR_PATH) private var gimbalJson
    @State private var pressing: CGPoint?
    @QgcPath(CAMERA_VIEW) private var cameraJson
    @QgcPath(SYNTHETIC_VIEW) private var syntheticJson
    @QgcBool(settingControl("settings.videoSettings.gridLines")) private var showGrid
    @QgcValue(settingControl("settings.videoSettings.videoFit")) private var fitMode
    @QgcValue(settingControl("settings.videoSettings.aspectRatio")) private var aspectSetting
    @ShownSecondCamera private var secondCamera

    var body: some View {
        let video = videoReading(videoJson)
        if video?.available == false && !(expanded && syntheticAvailable(syntheticJson)) {
            if expanded {
                NoVideoArea(underFlyChrome: expanded && !fullScreen, thumb: false, video: video)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(theme.colors.surfaceVariant)
            }
        } else {
            HStack(spacing: PANE_DIVIDER) {
                playing(video)
                if expanded, let second = secondCamera {
                    SecondPane(camera: second)
                }
            }
        }
    }

    private func playing(_ video: VideoReading?) -> some View {
        let decoding = video?.decoding == true
        let aspect = videoAspect(video?.sourceSize, aspectSetting.double)
        let holdDrives = onScreenGimbal(gimbalJson).map { $0.enabled && $0.clickAndDrag } == true
        let holds = expanded && decoding && onGround != nil && !holdDrives
        let hold = pictureHandlers(hasGimbal: gimbalJson?["shown"].bool == true)
        return GeometryReader { geometry in
            let shown = videoContentSize(geometry.size.width, geometry.size.height, aspect, fitMode.int ?? VIDEO_FIT_HEIGHT)
            ZStack {
                ZStack {
                    VideoChannelSurface(channel: MAIN_VIDEO_CHANNEL)
                    if decoding {
                        if showGrid && !fullScreen { VideoGrid() }
                        if expanded { VideoArOverlay() }
                        ProximityRadarOverlay()
                        ObstacleVideoOverlay(showText: expanded)
                        if expanded { GimbalScreenControl() }
                        DetectionOverlay()
                        TrackingBoxOverlay()
                        if expanded { VideoStatsPill() }
                        if expanded { OrbitOutline() }
                        HoldRing(at: holds ? pressing : nil)
                        if let spot = groundSpot { PictureMark(spot: spot) }
                    }
                }
                .frame(width: shown.0, height: shown.1)
                .contentShape(Rectangle())
                .pictureHold(holds, hold)
                if expanded && decoding {
                    LookAtReticle()
                    GimbalModeSwitch()
                        .padding(Space.s2)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                }
                if !decoding && syntheticAvailable(syntheticJson) {
                    SyntheticView(aimable: expanded, standIn: expanded ? video?.standIn : nil)
                } else if !decoding {
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

    private func pictureHandlers(hasGimbal: Bool) -> PictureHoldHandlers {
        let onGround = onGround
        return PictureHoldHandlers(
            onPressing: { pressing = $0 },
            onTap: onPictureTap,
            onHeld: { at, size in
                Task {
                    let press = await offMain { groundAt(at, size, hasGimbal: hasGimbal) }
                    onGround?(press)
                }
            }
        )
    }
}

private struct SecondPane: View {
    let camera: CameraEntry
    @Environment(\.theme) private var theme

    var body: some View {
        ZStack(alignment: .topLeading) {
            if camera.source == SYNTHETIC_SOURCE {
                SyntheticView()
            } else {
                VideoChannelSurface(channel: PIP_VIDEO_CHANNEL)
            }
            if camera.status != .Live {
                theme.aircast.outdoorBackground
                    .overlay(Text(camera.status.label).font(.labelLarge).foregroundStyle(theme.aircast.outdoorForeground))
            }
            HStack(spacing: Space.s1) {
                CameraStatusDot(status: camera.status)
                Text(camera.short).font(.labelMedium).foregroundStyle(theme.aircast.outdoorForeground).lineLimit(1)
            }
            .padding(Space.s2)
            .osdShadow()
            .allowsHitTesting(false)
        }
        .background(Color.black)
        .contentShape(Rectangle())
        .onTapGesture { showCamera(camera.slot) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Second camera, \(camera.title)")
        .accessibilityAddTraits(.isButton)
        .accessibilityAction(named: "Make \(camera.title) the main view") { showCamera(camera.slot) }
    }
}

private let PANE_DIVIDER: CGFloat = 2

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

struct DecodedFrame: Equatable {
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

// Shows one core video channel: decoded frames are copied off the core and set as the backing layer's contents on
// every display refresh. The same on iOS (UIView) and macOS (layer-backed NSView, as the native macOS head does).
final class VideoChannelLayerView: PlatformView {
    private let channel: Int32
    private var link: CADisplayLink?
    private var pipReported = false
    private var copying = false
    private var shownFrames: Int64 = -1

    init(channel: Int) {
        self.channel = Int32(channel)
        super.init(frame: .zero)
        #if os(iOS)
        backgroundColor = .black
        isUserInteractionEnabled = false
        layer.contentsGravity = .resizeAspect
        #elseif os(macOS)
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        layer?.contentsGravity = .resizeAspect
        #endif
    }

    required init?(coder: NSCoder) { nil }

    #if os(iOS)
    override func didMoveToWindow() {
        super.didMoveToWindow()
        windowChanged()
    }
    #elseif os(macOS)
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        windowChanged()
    }

    // Clicks and drags fall through to the SwiftUI gestures around the picture, as with isUserInteractionEnabled off.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    #endif

    private func windowChanged() {
        let onScreen = window != nil
        if onScreen { start() } else { stop() }
        guard channel == Int32(PIP_VIDEO_CHANNEL), onScreen != pipReported else { return }
        pipReported = onScreen
        if onScreen { pipSurfaces.created() } else { pipSurfaces.destroyed() }
    }

    private func start() {
        guard link == nil else { return }
        #if os(iOS)
        let made = CADisplayLink(target: FrameTicker(self), selector: #selector(FrameTicker.tick))
        #elseif os(macOS)
        let made = displayLink(target: FrameTicker(self), selector: #selector(FrameTicker.tick))
        #endif
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
                #if os(iOS)
                self.layer.contents = image
                #elseif os(macOS)
                self.layer?.contents = image
                #endif
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

struct VideoChannelSurface: PlatformViewRepresentable {
    typealias Coordinator = Void

    let channel: Int

    func makePlatformView(context: Context) -> VideoChannelLayerView { VideoChannelLayerView(channel: channel) }

    static func dismantlePlatformView(_ view: VideoChannelLayerView, coordinator: ()) { view.stop() }
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
