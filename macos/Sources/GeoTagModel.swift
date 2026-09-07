struct GeoTagJob: Equatable {
    var logFile = ""
    var imageDirectory = ""
    var saveDirectory = ""
    var errorMessage = ""
    var progress = 0.0
    var running = false

    static let taggedFolder = "TAGGED"

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
