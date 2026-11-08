import Foundation
import QGCCore

enum CoreHost {
    private static let organization = "Aircast"
    private static let defaultApplication = "Aircast QGC"
    private static let defaultDebugApiPort = "8799"
    private(set) static var started = false

    static func start() {
        guard !started else { return }
        started = true
        let files = FileManager.default
        let support = files.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let caches = files.urls(for: .cachesDirectory, in: .userDomainMask)[0]
        let documents = files.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let named = CommandLine.arguments.firstIndex(of: "--app-name").flatMap { CommandLine.arguments.dropFirst($0 + 1).first }
        let folder = support.appendingPathComponent("settings/\(named ?? organization)", isDirectory: true)
        try? files.createDirectory(at: folder, withIntermediateDirectories: true)
        let existing = (try? files.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil))?
            .filter { $0.pathExtension == "ini" }
            .min { $0.lastPathComponent < $1.lastPathComponent }
        let settings = existing ?? folder.appendingPathComponent("\(named ?? defaultApplication).ini")
        let application = settings.deletingPathExtension().lastPathComponent
        let mapCache = caches.appendingPathComponent("QGCMapCache", isDirectory: true)
        try? files.createDirectory(at: mapCache, withIntermediateDirectories: true)
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? ""
        let debug = isDebugBuild ? ["--port", debugApiPort, "--debug-build"] : []
        let arguments = [
            "Aircast",
            "--settings", settings.path,
            "--app-name", application,
            "--map-cache", mapCache.appendingPathComponent("qgcMapCache.db").path,
            "--save-path", documents.appendingPathComponent(application, isDirectory: true).path,
            "--app-version", version,
        ] + debug
        #if os(macOS)
        useBundledGStreamer()
        #endif
        qgc_ios_register_gstreamer_plugins(Bundle.main.path(forResource: "ca-certificates", ofType: "crt"))
        launch(arguments)
        GcsLocation.start()
        SpeechOut.start()
        VideoDriver.start()
    }

    static func handleDeepLink(_ url: URL) {
        offMain { qgc_handle_deep_link(url.absoluteString) }
    }

    @MainActor
    static func stop() {
        guard started else { return }
        GamepadInput.stop()
        VideoDriver.stop()
        GcsLocation.stop()
        SpeechOut.stop()
        qgc_shutdown()
    }

    private static var debugApiPort: String {
        ProcessInfo.processInfo.environment["QGC_DEBUG_API_PORT"] ?? defaultDebugApiPort
    }

    static var isDebugBuild: Bool {
        #if DEBUG
        true
        #else
        false
        #endif
    }

    #if os(macOS)
    /// A packaged app carries its own GStreamer.framework (ios/package-macos.sh), so GStreamer is pointed at the
    /// plugins, plugin scanner and GIO modules inside it rather than anything installed on the Mac, the way the
    /// Qt desktop app's GStreamerEnvironment did. A development build has none embedded and is left alone.
    private static func useBundledGStreamer() {
        guard let frameworks = Bundle.main.privateFrameworksURL else { return }
        let root = frameworks.appendingPathComponent("GStreamer.framework/Versions/1.0", isDirectory: true)
        let plugins = root.appendingPathComponent("lib/gstreamer-1.0", isDirectory: true).path
        guard FileManager.default.fileExists(atPath: plugins) else { return }
        let scanner = root.appendingPathComponent("libexec/gstreamer-1.0/gst-plugin-scanner").path
        let gioModules = root.appendingPathComponent("lib/gio/modules", isDirectory: true).path
        for name in ["GST_PLUGIN_PATH", "GST_PLUGIN_PATH_1_0", "GST_PLUGIN_SCANNER", "GST_PLUGIN_SCANNER_1_0", "GIO_EXTRA_MODULES"] {
            unsetenv(name)
        }
        setenv("GST_PLUGIN_SYSTEM_PATH_1_0", plugins, 1)
        setenv("GST_PLUGIN_SYSTEM_PATH", plugins, 1)
        setenv("GST_REGISTRY_REUSE_PLUGIN_SCANNER", "no", 1)
        setenv("GST_REGISTRY_FORK", "no", 1)
        if FileManager.default.isExecutableFile(atPath: scanner) {
            setenv("GST_PLUGIN_SCANNER_1_0", scanner, 1)
            setenv("GST_PLUGIN_SCANNER", scanner, 1)
        }
        if FileManager.default.fileExists(atPath: gioModules) {
            setenv("GIO_EXTRA_MODULES", gioModules, 1)
        }
        if let certificates = Bundle.main.path(forResource: "ca-certificates", ofType: "crt") {
            setenv("SSL_CERT_FILE", certificates, 0)
        }
    }
    #endif

    private static func launch(_ arguments: [String]) {
        var pointers = arguments.map { strdup($0) }
        defer { pointers.forEach { free($0) } }
        _ = pointers.withUnsafeMutableBufferPointer { buffer in
            qgc_start(Int32(buffer.count), buffer.baseAddress)
        }
    }
}
