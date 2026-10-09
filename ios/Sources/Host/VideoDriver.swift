import Foundation
import os
import QGCCore
import UIKit

private let RECORDING_TASK = "aircast:recording"

enum VideoDriver {
    private static let interval = 0.5
    private static let main = Int32(QGC_VIDEO_MAIN)
    private static let channels: [Int32] = [main, Int32(QGC_VIDEO_PIP)]
    private static let queue = DispatchQueue(label: "one.aircast.video-driver")
    private static let log = Logger(subsystem: "one.aircast.app", category: "Video")
    private static var timer: DispatchSourceTimer?
    private static var played: [Int32: Channel] = [:]
    private static var recording: JSON?
    private static var recordingReported = false
    private static var restarts: Set<Int32> = []

    private struct Channel {
        var driven: String?
        var restarted = false
        var error = ""
        var streamed = ""
        var decoding = false
    }

    static func start() {
        queue.async {
            guard timer == nil, qgc_video_available() else { return }
            VideoCommands.setNativeRendering(true)
            VideoCommands.initNative()
            let made = DispatchSource.makeTimerSource(queue: queue)
            made.schedule(deadline: .now() + interval, repeating: interval)
            made.setEventHandler(handler: step)
            made.resume()
            timer = made
        }
    }

    static func stop() {
        queue.sync {
            guard let running = timer else { return }
            running.cancel()
            timer = nil
            channels.forEach(qgc_video_stop)
        }
    }

    @discardableResult
    static func restart() -> Bool {
        queue.async { restarts.insert(main) }
        return VideoCommands.restart()
    }

    fileprivate static func finishRecording() {
        queue.sync { qgc_video_stop_recording(main) }
    }

    private static func step() {
        let view = Qgc.get(VIDEO_VIEW)
        let wanted: [Int32: String?] = [
            main: view["nativePipeline"].stringOrNil,
            Int32(QGC_VIDEO_PIP): view["pipPipeline"].stringOrNil,
        ]
        channels.forEach { stopChanged($0, wanted[$0] ?? nil) }
        channels.forEach { startWanted($0, wanted[$0] ?? nil) }
        record(view["nativeRecording"])
        channels.forEach(report)
    }

    private static func stopChanged(_ channel: Int32, _ wanted: String?) {
        let restart = restarts.remove(channel) != nil
        guard let driven = played[channel]?.driven, restart || wanted != driven else { return }
        qgc_video_stop(channel)
        log.info("Video channel \(channel) stopped")
        played[channel]?.driven = nil
    }

    private static func startWanted(_ channel: Int32, _ wanted: String?) {
        guard let pipeline = wanted, pipeline != played[channel]?.driven else { return }
        let started = qgc_video_start(channel, pipeline)
        let error = started ? "" : String(cString: qgc_video_last_error(channel))
        if started {
            log.info("Video channel \(channel) started")
        } else {
            log.warning("Video pipeline on channel \(channel) did not start: \(error, privacy: .public)")
        }
        played[channel] = Channel(driven: pipeline, restarted: true, error: error)
    }

    private static func record(_ wanted: JSON) {
        let asked = wanted.objectOrNil
        if asked != recording {
            if recording != nil { qgc_video_stop_recording(main) }
            if let asked { startRecording(asked) }
            recording = asked
        }
        let active = qgc_video_recording(main)
        guard active != recordingReported else { return }
        recordingReported = active
        onMain { RecordingKeepAlive.hold(active) }
        VideoCommands.reportRecording(active)
    }

    private static func startRecording(_ asked: JSON) {
        guard let file = asked["file"].stringOrNil, let format = asked["format"].int.flatMap(Int32.init(exactly:)) else { return }
        guard !qgc_video_start_recording(main, file, format) else { return }
        log.warning("Recording did not start: \(String(cString: qgc_video_last_error(main)), privacy: .public)")
    }

    private static func report(_ channel: Int32) {
        guard let state = played[channel], state.driven != nil else { return }
        let running = qgc_video_running(channel)
        let frames = qgc_video_frames(channel)
        let width = qgc_video_width(channel)
        let height = qgc_video_height(channel)
        if frames > 0 && !state.decoding {
            log.info("Video channel \(channel) decoding \(width)x\(height)")
        }
        let source = qgc_video_source_buffers(channel)
        let streamed = String(cString: qgc_video_stream_error(channel))
        if streamed != state.streamed && !streamed.isEmpty {
            log.warning("Video stream error on channel \(channel): \(streamed, privacy: .public)")
        }
        played[channel] = Channel(driven: state.driven, restarted: false, error: state.error, streamed: streamed, decoding: state.decoding || frames > 0)
        VideoCommands.reportNative(running, frames, width, height, state.error.ifEmpty(streamed), source, state.restarted, channel)
    }
}

@MainActor
private enum RecordingKeepAlive {
    static var task = UIBackgroundTaskIdentifier.invalid

    static func hold(_ recording: Bool) {
        guard recording else { return release() }
        guard task == .invalid else { return }
        task = UIApplication.shared.beginBackgroundTask(withName: RECORDING_TASK) {
            VideoDriver.finishRecording()
            release()
        }
    }

    static func release() {
        guard task != .invalid else { return }
        UIApplication.shared.endBackgroundTask(task)
        task = .invalid
    }
}
