import Foundation
import os
import QGCCore

enum VideoDriver {
    private static let interval = 0.5
    private static let channels: [Int32] = [Int32(QGC_VIDEO_MAIN), Int32(QGC_VIDEO_PIP)]
    private static let queue = DispatchQueue(label: "one.aircast.video-driver")
    private static let log = Logger(subsystem: "one.aircast.app", category: "Video")
    private static var timer: DispatchSourceTimer?
    private static var played: [Int32: Channel] = [:]
    private static var recording: JSON?
    private static var recordingReported = false

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
            Qgc.invoke("video.setNativeRendering", true)
            Qgc.invoke("video.initNative")
            let made = DispatchSource.makeTimerSource(queue: queue)
            made.schedule(deadline: .now() + interval, repeating: interval)
            made.setEventHandler(handler: step)
            made.resume()
            timer = made
        }
    }

    private static func step() {
        let view = Qgc.get("view.video")
        let wanted: [Int32: String?] = [
            Int32(QGC_VIDEO_MAIN): view["nativePipeline"].stringOrNil,
            Int32(QGC_VIDEO_PIP): view["pipPipeline"].stringOrNil,
        ]
        channels.forEach { stopChanged($0, wanted[$0] ?? nil) }
        channels.forEach { startWanted($0, wanted[$0] ?? nil) }
        record(view["nativeRecording"])
        channels.forEach(report)
    }

    private static func stopChanged(_ channel: Int32, _ wanted: String?) {
        guard let driven = played[channel]?.driven, wanted != driven else { return }
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
        let main = Int32(QGC_VIDEO_MAIN)
        let asked = wanted.object == nil ? nil : wanted
        if asked != recording {
            if recording != nil { qgc_video_stop_recording(main) }
            if let asked, let file = asked["file"].stringOrNil, let format = asked["format"].int {
                _ = qgc_video_start_recording(main, file, Int32(format))
            }
            recording = asked
        }
        let active = qgc_video_recording(main)
        guard active != recordingReported else { return }
        recordingReported = active
        Qgc.invoke("video.reportRecording", active)
    }

    private static func report(_ channel: Int32) {
        guard var state = played[channel], state.driven != nil else { return }
        let running = qgc_video_running(channel)
        let frames = qgc_video_frames(channel)
        let width = qgc_video_width(channel)
        let height = qgc_video_height(channel)
        if frames > 0 && !state.decoding {
            state.decoding = true
            log.info("Video channel \(channel) decoding \(width)x\(height)")
        }
        let source = qgc_video_source_buffers(channel)
        let streamed = String(cString: qgc_video_stream_error(channel))
        if streamed != state.streamed && !streamed.isEmpty {
            log.warning("Video stream error on channel \(channel): \(streamed, privacy: .public)")
        }
        state.streamed = streamed
        let error = state.error.isEmpty ? streamed : state.error
        let restarted = state.restarted
        state.restarted = false
        played[channel] = state
        Qgc.invoke("video.reportNative", running, frames, width, height, error, source, restarted, channel)
    }
}
