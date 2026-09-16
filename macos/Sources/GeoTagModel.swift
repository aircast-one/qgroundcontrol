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

    // The button's word and the Save-to placeholder were both spelled in AnalyzeWindow, which
    // swift-checks does not compile. RemoteSupport already names its own actionTitle for the same
    // job -- one button whose word follows a state -- so this is the second of a pair rather than
    // a new idea.
    //
    // "Try Again" is offered only after a FAILURE, never after a finished run: a completed tagging
    // has NOTHING TO RETRY. The reason first written here was that such a press would be inert --
    // startTagging on a QThread still running, a warning and no visible effect -- and that was
    // true for exactly one commit. 30b647386 connected taggingComplete to QThread::quit, so a
    // successful run now ends its thread and a Try Again there would start a SECOND RUN over
    // images already tagged. The word was right and its reason described a version.
    var actionTitle: String { failed ? "Try Again" : "Start Tagging" }

    // The placeholder is the DESTINATION, not an instruction: with images chosen it is where the
    // tagged copies will actually land, and only with nothing chosen at all is there a sentence
    // describing what would happen. Drawing the sentence once an image folder exists would hide
    // the one path the operator needs to check before a run that rewrites files.
    //
    // The call site shortens it unconditionally rather than branching, because shortPath returns a
    // string with fewer than three "/" components unchanged -- so the sentence passes through
    // untouched and only a real path is abbreviated. That is asserted rather than assumed: a
    // branch whose arms agree is one a later hand deletes the wrong half of.
    var destinationPlaceholder: String {
        imageDirectory.isEmpty ? "A \(GeoTagJob.taggedFolder) folder beside your images" : destination
    }

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
