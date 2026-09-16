struct GeoTagJob: Equatable {
    var logFile = ""
    var imageDirectory = ""
    var saveDirectory = ""
    var errorMessage = ""
    var progress = 0.0
    var running = false

    static let taggedFolder = "TAGGED"

    // GeoTagWorker.cc:142 routes ONLY a lower-case ".ulg" to the ULog parser and sends everything
    // else to PX4LogParser, which has no failure path: getTagsFromLog returns true whatever the
    // bytes were (PX4LogParser.cc:106). An unparsable log therefore reports a SUCCESSFUL parse with
    // an empty trigger list, and the operator is then told "Calibration failed: No triggers or
    // images available" -- a sentence about their images, for a log format nothing in this tree
    // reads. This picker used to offer "bin" as a third type. ArduPilot dataflash IS .bin and this
    // fork flies ArduPilot, so that was the likely choice, not an edge one, and no parser reads it.
    // The two left are exactly the two QGC offers (GeoTagPage.qml:74). Picking any other file is
    // still allowed -- the panel sets allowsOtherFileTypes, as QGC's "All Files (*)" does -- so
    // this narrows what is ADVERTISED as supported, not what can be chosen.
    static let logExtensions = ["ulg", "px4log"]

    var canStart: Bool { !logFile.isEmpty && !imageDirectory.isEmpty }

    var destination: String {
        if !saveDirectory.isEmpty { return saveDirectory }
        if !imageDirectory.isEmpty { return imageDirectory + "/" + GeoTagJob.taggedFolder }
        return ""
    }

    var progressText: String {
        running || progress > 0 ? String(format: "%.0f%%", progress) : ""
    }

    var busy: Bool { running && errorMessage.isEmpty }

    var failed: Bool { running && !errorMessage.isEmpty }

    var finished: Bool { !running && progress >= 100 }

    static func shortPath(_ path: String, home: String) -> String {
        let abbreviated = abbreviate(path, home: home)
        let parts = abbreviated.split(separator: "/", omittingEmptySubsequences: true)
        guard parts.count > 2 else { return abbreviated }
        return "\u{2026}/" + parts.suffix(2).joined(separator: "/")
    }

    static func abbreviate(_ path: String, home: String) -> String {
        guard !home.isEmpty, path == home || path.hasPrefix(home + "/") else { return path }
        return "~" + path.dropFirst(home.count)
    }
}
