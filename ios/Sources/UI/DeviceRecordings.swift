import Foundation

struct RecordingsLimit: Equatable {
    let value: String
    let options: [DeviceOption]
    let used: String
}

struct RecordCamera: Equatable {
    let name: String
    let recording: Bool
    let note: String
}

struct RecordingSegment: Equatable {
    var at: String
    var started: String
    var duration: String
    var size: String
    var url: String
    var open: Bool
    var noTelemetry: Bool
}

struct RecordedCamera: Equatable {
    let name: String
    let status: DeviceBadge
    let summary: String
    let full: Bool
    let usage: String
    let segments: [RecordingSegment]
}

struct RecordingsView: Equatable {
    let limit: RecordingsLimit
    let cameras: [RecordCamera]
    let recorded: [RecordedCamera]
}

private func parseSegment(_ json: JSON) -> RecordingSegment {
    RecordingSegment(
        at: json["at"].string,
        started: json["started"].string,
        duration: json["duration"].string,
        size: json["size"].string,
        url: json["url"].string,
        open: json["open"].bool,
        noTelemetry: json["noTelemetry"].bool
    )
}

private func parseRecorded(_ json: JSON) -> RecordedCamera {
    RecordedCamera(
        name: json["name"].string,
        status: parseBadge(json["status"].objectOrNil),
        summary: json["summary"].string,
        full: json["full"].bool,
        usage: json["usage"].string,
        segments: json["segments"].objects.map(parseSegment)
    )
}

func parseRecordings(_ json: JSON) -> RecordingsView? {
    guard let limit = json["limit"].objectOrNil else { return nil }
    return RecordingsView(
        limit: RecordingsLimit(value: limit["value"].string, options: parseOptions(limit, "options"), used: limit["used"].string),
        cameras: json["cameras"].objects.map { RecordCamera(name: $0["name"].string, recording: $0["recording"].bool, note: $0["note"].string) },
        recorded: json["recorded"].objects.map(parseRecorded)
    )
}

func lowersLimit(_ current: String, _ picked: String) -> Bool {
    guard let next = Int64(picked), let now = Int64(current) else { return false }
    return next < now
}

func recordPath(_ camera: String) -> String { "/api/streams/\(uriComponent(camera))/record" }

func recordingsPath(_ camera: String) -> String { "/api/recordings/\(uriComponent(camera))" }

func segmentPath(_ camera: String, _ segment: RecordingSegment) -> String { "\(recordingsPath(camera))?start=\(uriComponent(segment.at))" }

func segmentDetail(_ segment: RecordingSegment) -> [String] {
    [
        segment.duration.isEmpty ? nil : segment.duration,
        segment.size,
        segment.noTelemetry ? "no flight data" : nil,
        segment.open ? "downloadable once it closes" : nil,
    ].compactMap { $0 }
}
