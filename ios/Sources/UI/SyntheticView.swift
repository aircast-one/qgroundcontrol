import SwiftUI
import UniformTypeIdentifiers
import WebKit

let SYNTHETIC_VIEW = "view.syntheticView"
let SYNTHETIC_OVERLAYS = "view.syntheticOverlays"
let SYNTHETIC_AIM = "syntheticView.aim"
let SYNTHETIC_LABEL = "Synthetic view"
let SYNTHETIC_SOURCE = "Synthetic View"
private let SYNTHETIC_SCHEME = "aircast"
private let SYNTHETIC_PAGE = "aircast://app/synthetic/index.html"
private let SYNTHETIC_FOLDER = "/synthetic/"
private let SYNTHETIC_TILES = "/synthetic/tiles"
private let SYNTHETIC_CESIUM = "/synthetic/Cesium/"
private let TILT_SPAN_DEG = 90.0
private let TILT_UP_DEG = 30.0
let DEFAULT_TILT_DEG = -15.0
private let PAN_SPAN_DEG = 360.0
private let PNG_SIGNATURE: [UInt8] = [0x89, 0x50, 0x4E, 0x47]
private let HTTP_OK = 200
private let HTTP_NOT_FOUND = 404
private let STAND_IN_ANNOUNCED_MS = 4000
private let EDGE_NOTE_SCRIM_ALPHA = 0.35
private let EDGE_NOTE_TEXT_ALPHA = 0.75
private let EDGE_NOTE_PAD_VERTICAL: CGFloat = 2

enum SyntheticDraw {
    case Loading, Drawn, Failed

    var note: String? {
        switch self {
        case .Loading: "Loading 3D view…"
        case .Drawn: nil
        case .Failed: "3D view failed to start"
        }
    }
}

func syntheticAvailable(_ view: JSON?) -> Bool { view?["available"].bool == true }

func syntheticChosen(_ reading: CamerasReading?) -> Bool { reading?.cameras.first(where: \.active)?.source == SYNTHETIC_SOURCE }

func syntheticFree(_ view: JSON?, picked: Bool) -> Bool {
    syntheticAvailable(view) && view?["locksOnTarget"].bool != true && (picked || view?["gimbalAimed"].bool != true)
}

func syntheticAims(_ view: JSON?, _ cameras: CamerasReading?) -> Bool { syntheticChosen(cameras) && syntheticFree(view, picked: true) }

func syntheticPoseScript(_ view: JSON) -> String { "window.aircast && window.aircast.pose(\(view.text))" }

func syntheticOverlaysScript(_ view: JSON) -> String { "window.aircast && window.aircast.overlays(\(view.text))" }

func syntheticTilePath(_ path: String) -> String? {
    path.hasPrefix(SYNTHETIC_TILES + "/") ? String(path.dropFirst(SYNTHETIC_TILES.count)) : nil
}

func syntheticBundlePath(_ path: String) -> String? {
    switch true {
    case path.hasPrefix(SYNTHETIC_CESIUM): "Cesium/" + path.dropFirst(SYNTHETIC_CESIUM.count)
    case path.hasPrefix(SYNTHETIC_FOLDER): String(path.dropFirst())
    default: nil
    }
}

func syntheticTilt(_ from: Double, _ dragged: Double, _ height: Double) -> Double {
    min(TILT_UP_DEG, max(-TILT_SPAN_DEG, from + dragged / max(height, 1) * TILT_SPAN_DEG))
}

func syntheticPan(_ from: Double, _ dragged: Double, _ width: Double) -> Double {
    let turned = (from + dragged / max(width, 1) * PAN_SPAN_DEG + 180).truncatingRemainder(dividingBy: 360)
    return (turned < 0 ? turned + 360 : turned) - 180
}

private func mimeType(_ file: URL) -> String {
    file.pathExtension == "js" ? "text/javascript" : UTType(filenameExtension: file.pathExtension)?.preferredMIMEType ?? "application/octet-stream"
}

private func tileMimeType(_ tile: Data) -> String {
    tile.starts(with: PNG_SIGNATURE) ? "image/png" : "image/jpeg"
}

func syntheticWebView(_ delegate: WKNavigationDelegate) -> WKWebView {
    let configuration = WKWebViewConfiguration()
    configuration.setURLSchemeHandler(SyntheticSchemeHandler(), forURLScheme: SYNTHETIC_SCHEME)
    let web = WKWebView(frame: .zero, configuration: configuration)
    web.isOpaque = false
    web.backgroundColor = .black
    web.scrollView.isScrollEnabled = false
    web.isUserInteractionEnabled = false
    web.isInspectable = CoreHost.isDebugBuild
    web.navigationDelegate = delegate
    URL(string: SYNTHETIC_PAGE).map { web.load(URLRequest(url: $0)) }
    return web
}

private final class SyntheticSchemeHandler: NSObject, WKURLSchemeHandler {
    private var tickets: [ObjectIdentifier: UInt64] = [:]

    func webView(_ webView: WKWebView, start task: any WKURLSchemeTask) {
        guard let url = task.request.url else { return }
        let encoded = URLComponents(url: url, resolvingAgainstBaseURL: false)?.percentEncodedPath ?? url.path
        if let address = syntheticTilePath(encoded).flatMap(tileAddress) {
            let id = ObjectIdentifier(task)
            tickets[id] = MapTileHost.fetch(address) { [weak self] data in
                DispatchQueue.main.async {
                    guard let self, self.tickets.removeValue(forKey: id) != nil else { return }
                    self.reply(task, url, data, data.map(tileMimeType) ?? "image/png")
                }
            }
            return
        }
        let file = syntheticBundlePath(url.path).flatMap { Bundle.main.resourceURL?.appendingPathComponent($0) }
        reply(task, url, file.flatMap { try? Data(contentsOf: $0) }, file.map(mimeType) ?? "application/octet-stream")
    }

    func webView(_ webView: WKWebView, stop task: any WKURLSchemeTask) {
        guard let ticket = tickets.removeValue(forKey: ObjectIdentifier(task)) else { return }
        MapTileHost.cancel(ticket)
    }

    private func reply(_ task: any WKURLSchemeTask, _ url: URL, _ data: Data?, _ mime: String) {
        guard let response = HTTPURLResponse(url: url, statusCode: data == nil ? HTTP_NOT_FOUND : HTTP_OK, httpVersion: "HTTP/1.1", headerFields: ["Content-Type": mime]) else { return }
        task.didReceive(response)
        task.didReceive(data ?? Data())
        task.didFinish()
    }
}

private struct SyntheticWebView: UIViewRepresentable {
    let pose: JSON?
    let overlays: JSON?

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let web = syntheticWebView(context.coordinator)
        context.coordinator.web = web
        return web
    }

    func updateUIView(_ web: WKWebView, context: Context) {
        context.coordinator.show(pose: pose, overlays: overlays)
    }

    static func dismantleUIView(_ web: WKWebView, coordinator: Coordinator) {
        web.stopLoading()
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        weak var web: WKWebView?
        private var loaded = false
        private var pose: JSON?
        private var overlays: JSON?
        private var sent: JSON?

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            loaded = true
            push()
        }

        func show(pose nextPose: JSON?, overlays nextOverlays: JSON?) {
            pose = nextPose
            overlays = nextOverlays
            push()
        }

        private func push() {
            guard loaded, let web else { return }
            if let pose { web.evaluateJavaScript(syntheticPoseScript(pose)) }
            guard let overlays, overlays != sent else { return }
            sent = overlays
            web.evaluateJavaScript(syntheticOverlaysScript(overlays))
        }
    }
}

struct ShownNote: Equatable {
    let text: String
    let atEdge: Bool
}

func shownNote(_ draw: SyntheticDraw, _ standIn: String?, announced: Bool) -> ShownNote? {
    draw.note.map { ShownNote(text: $0, atEdge: false) } ?? standIn.map { ShownNote(text: $0, atEdge: announced) }
}

private struct NativePage: View {
    let view: JSON?
    let standIn: String?

    var body: some View {
        let draw = nativeDraw(view)
        ZStack {
            SyntheticSurface()
            SyntheticNote(draw: draw, standIn: standIn)
        }
    }
}

private struct SyntheticNote: View {
    let draw: SyntheticDraw
    let standIn: String?
    @State private var announced = false
    @Environment(\.theme) private var theme

    private var standingIn: Bool { draw.note == nil && standIn != nil }

    var body: some View {
        ZStack {
            Color.clear
            if let note = shownNote(draw, standIn, announced: announced) {
                if note.atEdge {
                    Text(SYNTHETIC_LABEL).font(.labelSmall).foregroundStyle(theme.aircast.outdoorForeground.opacity(EDGE_NOTE_TEXT_ALPHA)).lineLimit(1)
                        .padding(.horizontal, Space.s2)
                        .padding(.vertical, EDGE_NOTE_PAD_VERTICAL)
                        .background(Color.black.opacity(EDGE_NOTE_SCRIM_ALPHA), in: Capsule())
                        .accessibilityElement(children: .ignore)
                        .accessibilityLabel(note.text)
                        .padding(Space.s3)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                } else {
                    HStack(spacing: Space.s2) {
                        if draw == .Loading { ProgressView().controlSize(.small).tint(theme.aircast.outdoorForeground) }
                        Text(note.text).font(.labelLarge).foregroundStyle(theme.aircast.outdoorForeground).multilineTextAlignment(.center).lineLimit(2)
                    }
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(note.text)
                    .padding(.horizontal, Space.s4)
                    .osdShadow()
                }
            }
        }
        .allowsHitTesting(false)
        .task(id: standingIn) {
            announced = false
            guard standingIn, (try? await Task.sleep(for: .milliseconds(STAND_IN_ANNOUNCED_MS))) != nil else { return }
            announced = true
        }
    }
}

struct SyntheticView: View {
    var aimable = false
    var standIn: String?
    @QgcPath(SYNTHETIC_VIEW) private var view
    @QgcPath(SYNTHETIC_OVERLAYS) private var overlays
    @ShownCameras private var cameras
    @State private var dragStart: (tilt: Double, pan: Double)?
    @State private var sent: (tilt: Double, pan: Double)?

    private var canAim: Bool { aimable && syntheticFree(view, picked: syntheticChosen(cameras)) }

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                if nativeChosen(view) {
                    NativePage(view: view, standIn: standIn)
                } else {
                    SyntheticWebView(pose: view, overlays: overlays)
                    SyntheticNote(draw: .Drawn, standIn: standIn)
                }
            }
            .contentShape(Rectangle())
            .gesture(aiming(geometry.size), including: canAim ? .all : .none)
            .simultaneousGesture(TapGesture(count: 2).onEnded { if canAim { aim(DEFAULT_TILT_DEG, 0) } }, including: canAim ? .all : .none)
        }
    }

    private func aiming(_ size: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 4)
            .onChanged { drag in
                let start = dragStart ?? sent ?? (view?["tilt"].double ?? DEFAULT_TILT_DEG, view?["pan"].double ?? 0)
                dragStart = start
                aim(syntheticTilt(start.tilt, drag.translation.height, size.height), syntheticPan(start.pan, drag.translation.width, size.width))
            }
            .onEnded { _ in dragStart = nil }
    }

    private func aim(_ tilt: Double, _ pan: Double) {
        sent = (tilt, pan)
        offMainInOrder { _ = Qgc.invoke(SYNTHETIC_AIM, tilt, pan) }
    }
}
