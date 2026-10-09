import SwiftUI
import UniformTypeIdentifiers

let LOG_REPLAY_VIEW = "view.logReplay"
let LOG_REPLAY_START = "logReplay.start"
let LOG_REPLAY_TOGGLE = "logReplay.togglePlay"
let LOG_REPLAY_SPEED = "logReplay.speed"
let LOG_REPLAY_SEEK = "logReplay.seek"
let LOG_REPLAY_CLOSE = "logReplay.close"
private let REPLAY_FOLDER = "log-replay"
private let REPLAY_FALLBACK = "log-replay.tlog"

func replayFileName(_ shown: String?) -> String {
    guard let shown else { return REPLAY_FALLBACK }
    let name = shown.components(separatedBy: "/").last ?? shown
    return name.isBlank || name == "." || name == ".." ? REPLAY_FALLBACK : name
}

private let PLAYING_POLL_MS = 250
private let IDLE_POLL_MS = 1000

struct LogReplay: Equatable {
    var shown: Bool
    var loaded: Bool
    var playing: Bool
    var percent: Float
    var playheadTime: String
    var totalTime: String
    var speedIndex: Int
    var speeds: [String]
    var canLoad: Bool
    var loadRefusal: String
    var error: String
}

func replayProgress(_ replay: LogReplay) -> String {
    [replay.playheadTime, replay.totalTime].filter { !$0.isBlank }.joined(separator: " of ")
}

func speedLabel(_ label: String) -> String { label.removingSuffix("x") + "×" }

func logReplay(_ view: JSON?) -> LogReplay? {
    guard let view, view["available"].bool else { return nil }
    return LogReplay(
        shown: view["shown"].bool,
        loaded: view["loaded"].bool,
        playing: view["playing"].bool,
        percent: Float(view["percent"].double(0)),
        playheadTime: view["playheadTime"].string,
        totalTime: view["totalTime"].string,
        speedIndex: view["speedIndex"].int(3),
        speeds: view["speeds"].array.map(\.string),
        canLoad: view["canLoad"].bool,
        loadRefusal: view["loadRefusal"].string,
        error: view["error"].string
    )
}

struct ReplayScrub: Equatable {
    private(set) var editing = false
    private(set) var dragged: Double?

    func shown(_ percent: Float) -> Double { dragged ?? Double(percent) }

    mutating func move(_ value: Double) -> Double? {
        guard editing else { return value }
        dragged = value
        return nil
    }

    mutating func edit(_ on: Bool) -> Double? {
        let seek = on ? nil : dragged
        editing = on
        dragged = on ? dragged : nil
        return seek
    }
}

private func stagedReplay(_ chosen: URL) -> String? {
    let scoped = chosen.startAccessingSecurityScopedResource()
    defer { if scoped { chosen.stopAccessingSecurityScopedResource() } }
    let files = FileManager.default
    let folder = files.temporaryDirectory.appendingPathComponent(REPLAY_FOLDER, isDirectory: true)
    try? files.removeItem(at: folder)
    guard (try? files.createDirectory(at: folder, withIntermediateDirectories: true)) != nil else { return nil }
    let staged = folder.appendingPathComponent(replayFileName(chosen.lastPathComponent))
    return (try? files.copyItem(at: chosen, to: staged)).map { staged.path }
}

struct LogReplayBar: View {
    @Environment(\.theme) private var theme
    @State private var read: LogReplay?
    @State private var refresh = 0
    @State private var scrub = ReplayScrub()
    @State private var message: String?
    @State private var picking = false

    var body: some View {
        ZStack(alignment: .topLeading) {
            if let replay = read, replay.shown {
                bar(replay)
            }
        }
        .task(id: refresh) {
            let now = await offMain { logReplay(Qgc.get(LOG_REPLAY_VIEW)) }
            read = now
            guard (try? await Task.sleep(for: .milliseconds(now?.playing == true ? PLAYING_POLL_MS : IDLE_POLL_MS))) != nil else { return }
            refresh += 1
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.item]) { result in
            switch result {
            case .success(let chosen):
                Task {
                    message = await offMain {
                        stagedReplay(chosen).map { Qgc.refusalOf(LOG_REPLAY_START, $0) } ?? "That file could not be read."
                    }
                    refresh += 1
                }
            case .failure(let error):
                message = error.localizedDescription
            }
        }
        .alert("Log replay", isPresented: Binding(get: { message != nil }, set: { shown in if !shown { message = nil } })) {
            Button("OK") { message = nil }
        } message: {
            Text(message ?? "")
        }
    }

    private func act(_ path: String, _ args: Any?...) {
        let sent = args.map(JSON.init)
        Task {
            message = await offMain { Qgc.refusalOf(path, arguments: sent) }
            refresh += 1
        }
    }

    private func bar(_ replay: LogReplay) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(replayProgress(replay)).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                Text("\(Int(replay.percent))%").font(.labelLarge).foregroundStyle(theme.colors.primary)
            }
            Slider(
                value: Binding(get: { scrub.shown(replay.percent) }, set: { value in
                    scrub.move(value).map { act(LOG_REPLAY_SEEK, $0) }
                }),
                in: 0...100
            ) { editing in
                scrub.edit(editing).map { act(LOG_REPLAY_SEEK, $0) }
            }
            .disabled(!replay.loaded)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(Array(replay.speeds.enumerated()), id: \.offset) { index, label in
                        CameraChip(label: speedLabel(label), selected: index == replay.speedIndex) { act(LOG_REPLAY_SPEED, index) }
                    }
                }
            }
            HStack(spacing: 8) {
                Spacer()
                Button("Close") { act(LOG_REPLAY_CLOSE) }.buttonStyle(.borderless)
                if !replay.loaded {
                    Button("Load telemetry log") {
                        if replay.canLoad { picking = true } else { message = replay.loadRefusal }
                    }
                    .buttonStyle(.bordered)
                }
                Button(replay.playing ? "Pause" : "Play") { act(LOG_REPLAY_TOGGLE) }
                    .buttonStyle(.filled)
                    .disabled(!replay.loaded)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity)
        .background(theme.colors.surfaceContainer)
    }
}
