import Foundation
import Observation

private let GEOTAG_POLL_MS = 250
let DEFAULT_GEOTAG_OUTPUT = "TAGGED"

func geoTagOutputText(_ output: String?, _ images: String?) -> String {
    output ?? images.map { "\($0)/\(DEFAULT_GEOTAG_OUTPUT)" } ?? "Default: /\(DEFAULT_GEOTAG_OUTPUT) subfolder"
}

@MainActor
@Observable
final class GeoTagRun {
    static let shared = GeoTagRun()

    var state: GeoTagState?
    var note: String?
    var busy = false
    var imageTree: URL?
    var outputTree: URL?
    @ObservationIgnored private var files = false

    func refresh() async {
        state = await offMain { geoTagState(Qgc.get(GEOTAG_ROOT)) }
    }

    func pickDownloadedLog(_ path: String) {
        note = nil
        set("logFile", path)
    }

    func set(_ path: String, _ value: any Sendable) {
        Task {
            _ = await offMain { Qgc.set("\(GEOTAG_ROOT).\(path)", value) }
            await refresh()
        }
    }

    private func withFiles(_ work: @escaping @MainActor () async -> Void) {
        guard !files else { return }
        files = true
        busy = true
        Task {
            await work()
            busy = false
            files = false
        }
    }

    private func keep(_ tree: URL) {
        _ = tree.startAccessingSecurityScopedResource()
    }

    func pickLog(_ uri: URL) {
        withFiles { [self] in
            let staged = await offMain { stageLog(uri, documentName(uri)) }
            note = staged == nil ? "That file could not be read." : nil
            if let staged { _ = await offMain { Qgc.set("\(GEOTAG_ROOT).logFile", staged) } }
            await refresh()
        }
    }

    func pickImages(_ tree: URL) {
        withFiles { [self] in
            keep(tree)
            imageTree = tree
            let outputChosen = outputTree != nil
            let (path, count, alreadyTagged) = await offMain {
                let (path, count) = stageImages(tree)
                return (path, count, !outputChosen && count > 0 && hasTaggedFolder(tree))
            }
            note = count == 0 ? "That folder holds no JPEG, TIFF or DNG images." : alreadyTagged ? GEOTAG_ALREADY_TAGGED : nil
            _ = await offMain { Qgc.set("\(GEOTAG_ROOT).imageDirectory", path) }
            await refresh()
        }
    }

    func pickOutput(_ tree: URL) {
        withFiles { [self] in
            keep(tree)
            outputTree = tree
            note = await offMain { holdsImages(tree) } ? GEOTAG_SAVE_HAS_IMAGES : nil
        }
    }

    func cancel() {
        Task {
            _ = await offMain { Qgc.invoke("\(GEOTAG_ROOT).cancelTagging") }
            await refresh()
        }
    }

    func start() {
        withFiles { [self] in
            note = nil
            let output = await offMain {
                let output = taggedOutputDir()
                Qgc.set("\(GEOTAG_ROOT).saveDirectory", output.path)
                Qgc.invoke("\(GEOTAG_ROOT).startTagging")
                return output
            }
            await refresh()
            while state?.inProgress == true {
                try? await Task.sleep(for: .milliseconds(GEOTAG_POLL_MS))
                await refresh()
            }
            let subfolder = outputTree == nil ? DEFAULT_GEOTAG_OUTPUT : nil
            guard let finished = state, !finished.previewMode, finished.tagged > 0, let target = outputTree ?? imageTree else { return }
            let published = await offMain { publishTagged(output, target, subfolder) }
            note = publishedNote(published, finished.tagged)
        }
    }
}

func publishedNote(_ published: Int, _ tagged: Int) -> String? {
    published == tagged ? nil : "Only \(published) of the \(tagged) tagged images could be written to the chosen folder."
}

private let GEOTAG_OFFSET_LIMIT = 3600.0

func parsedOffset(_ typed: String) -> Double? {
    let text = typed.trimmed.replacingOccurrences(of: ",", with: ".")
    if text.isEmpty { return 0 }
    let decimals = text.firstIndex(of: ".").map { text[text.index(after: $0)...].count } ?? 0
    return Double(text).flatMap { (-GEOTAG_OFFSET_LIMIT...GEOTAG_OFFSET_LIMIT).contains($0) && decimals <= 1 ? $0 : nil }
}

let GEOTAG_ALREADY_TAGGED = "Images have already been tagged. Existing images will be removed."
let GEOTAG_SAVE_HAS_IMAGES = "The save folder already contains images."

func shownOffset(_ seconds: Double) -> String { String(format: "%.1f", seconds) }
