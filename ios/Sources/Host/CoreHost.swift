import Foundation
import QGCCore

enum CoreHost {
    private static let organization = "Aircast"
    private static let defaultApplication = "Aircast QGC"
    private static let debugApiPort = "8799"
    private(set) static var started = false

    static func start() {
        guard !started else { return }
        started = true
        let files = FileManager.default
        let support = files.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let caches = files.urls(for: .cachesDirectory, in: .userDomainMask)[0]
        let documents = files.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let folder = support.appendingPathComponent("settings/\(organization)", isDirectory: true)
        try? files.createDirectory(at: folder, withIntermediateDirectories: true)
        let existing = (try? files.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil))?
            .filter { $0.pathExtension == "ini" }
            .min { $0.lastPathComponent < $1.lastPathComponent }
        let settings = existing ?? folder.appendingPathComponent("\(defaultApplication).ini")
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

    static var isDebugBuild: Bool {
        #if DEBUG
        true
        #else
        false
        #endif
    }

    private static func launch(_ arguments: [String]) {
        var pointers = arguments.map { strdup($0) }
        defer { pointers.forEach { free($0) } }
        _ = pointers.withUnsafeMutableBufferPointer { buffer in
            qgc_start(Int32(buffer.count), buffer.baseAddress)
        }
    }
}
