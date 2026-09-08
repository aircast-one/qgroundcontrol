import AppKit
import Foundation
import UniformTypeIdentifiers

final class GeoTagStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?

    static let labels = ["logFile": "the flight log",
                         "imageDirectory": "the image folder",
                         "saveDirectory": "the destination folder"]
    static let probeID = "geoTag"

    @Published private(set) var job = GeoTagJob()

    private var poll: Timer?

    func reload() {
        let controller = Bridge.group("geoTag")
        guard controller["kind"] as? String == "object" else { return }

        var reading = GeoTagJob()
        reading.logFile = (controller["logFile"] as? String) ?? ""
        reading.imageDirectory = (controller["imageDirectory"] as? String) ?? ""
        reading.saveDirectory = (controller["saveDirectory"] as? String) ?? ""
        reading.errorMessage = (controller["errorMessage"] as? String) ?? ""
        reading.progress = (controller["progress"] as? NSNumber)?.doubleValue ?? 0
        reading.running = (controller["inProgress"] as? NSNumber)?.boolValue ?? false

        if reading != job { job = reading }
        reading.busy ? startPolling() : stopPolling()
    }

    func chooseLogFile() {
        let panel = NSOpenPanel()
        panel.title = "Select the flight log"
        panel.allowedContentTypes = ["ulg", "px4log", "bin"].compactMap { UTType(filenameExtension: $0) }
        panel.allowsOtherFileTypes = true
        panel.canChooseDirectories = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        set("logFile", url.path)
    }

    func chooseImageDirectory() {
        chooseDirectory("Select the folder holding the images", into: "imageDirectory")
    }

    func chooseSaveDirectory() {
        chooseDirectory("Select where the tagged images go", into: "saveDirectory")
    }

    private func chooseDirectory(_ title: String, into property: String) {
        let panel = NSOpenPanel()
        panel.title = title
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        guard panel.runModal() == .OK, let url = panel.url else { return }
        set(property, url.path)
    }

    func set(_ property: String, _ path: String) {
        write("geoTag.\(property)", path, GeoTagStore.labels[property] ?? property)
        reload()
    }

    func start() {
        guard job.canStart, !job.busy else { return }
        if job.failed {
            Bridge.invoke("geoTag.cancelTagging")
        }
        Bridge.invoke("geoTag.startTagging")
        reload()
        startPolling()
    }

    func cancel() {
        guard job.running else { return }
        Bridge.invoke("geoTag.cancelTagging")
        reload()
    }

    private func startPolling() {
        guard poll == nil else { return }
        poll = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) { [weak self] _ in
            self?.reload()
        }
    }

    private func stopPolling() {
        poll?.invalidate()
        poll = nil
    }

    func probeState() -> [String: Any] {
        ["writeFailure": writeFailure ?? "",
         "logFile": job.logFile, "imageDirectory": job.imageDirectory,
         "saveDirectory": job.saveDirectory, "destination": job.destination,
         "error": job.errorMessage, "progress": job.progress,
         "running": job.running, "canStart": job.canStart,
         "busy": job.busy, "failed": job.failed]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
        case "setLogFile": set("logFile", args["path"] ?? "")
        case "setImageDirectory": set("imageDirectory", args["path"] ?? "")
        case "setSaveDirectory": set("saveDirectory", args["path"] ?? "")
        case "start":
            guard job.canStart else {
                return ["ok": false, "error": "a log file and an image folder are needed first"]
            }
            start()
        case "cancel": cancel()
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
