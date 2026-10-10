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
private let DEFAULT_TILT_DEG = -15.0
private let PAN_SPAN_DEG = 360.0
private let PNG_SIGNATURE: [UInt8] = [0x89, 0x50, 0x4E, 0x47]
private let HTTP_OK = 200
private let HTTP_NOT_FOUND = 404

func syntheticAvailable(_ view: JSON?) -> Bool { view?["available"].bool == true }

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
    min(0, max(-TILT_SPAN_DEG, from + dragged / max(height, 1) * TILT_SPAN_DEG))
}

func syntheticPan(_ from: Double, _ dragged: Double, _ width: Double) -> Double {
    let turned = (from - dragged / max(width, 1) * PAN_SPAN_DEG + 180).truncatingRemainder(dividingBy: 360)
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
        if let pose, syntheticAvailable(pose) { web.evaluateJavaScript(syntheticPoseScript(pose)) }
        context.coordinator.show(overlays)
    }

    static func dismantleUIView(_ web: WKWebView, coordinator: Coordinator) {
        web.stopLoading()
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        weak var web: WKWebView?
        private var loaded = false
        private var overlays: JSON?
        private var sent: JSON?

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            loaded = true
            push()
        }

        func show(_ next: JSON?) {
            overlays = next
            push()
        }

        private func push() {
            guard loaded, let web, let overlays, overlays != sent else { return }
            sent = overlays
            web.evaluateJavaScript(syntheticOverlaysScript(overlays))
        }
    }
}

struct SyntheticView: View {
    var aimable = false
    @QgcPath(SYNTHETIC_VIEW) private var view
    @QgcPath(SYNTHETIC_OVERLAYS) private var overlays
    @State private var dragStart: (tilt: Double, pan: Double)?
    @State private var sent: (tilt: Double, pan: Double)?

    private var canAim: Bool { aimable && view?["aimable"].bool == true }

    var body: some View {
        GeometryReader { geometry in
            SyntheticWebView(pose: view, overlays: overlays)
            .contentShape(Rectangle())
            .gesture(aiming(geometry.size), including: canAim ? .all : .none)
            .simultaneousGesture(TapGesture(count: 2).onEnded { if canAim { aim(DEFAULT_TILT_DEG, 0) } }, including: canAim ? .all : .none)
        }
    }

    private func aiming(_ size: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 4)
            .onChanged { drag in
                let start = dragStart ?? sent ?? (view?["pitch"].double ?? DEFAULT_TILT_DEG, view?["pan"].double ?? 0)
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
