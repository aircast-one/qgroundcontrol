import AVFoundation
import Foundation

struct BufferedReading: Equatable {
    var running = false
    var frames: Int64 = 0
    var width: Int32 = 0
    var height: Int32 = 0
    var error = ""
}

let BUFFERED_PROGRESS_PER_SECOND = 30.0

func bufferedReading(ready: Bool, seconds: Double, size: CGSize, error: String?) -> BufferedReading {
    let played = seconds.isFinite && seconds > 0 ? seconds : 0
    return BufferedReading(
        running: ready,
        frames: ready ? Int64(played * BUFFERED_PROGRESS_PER_SECOND) : 0,
        width: Int32(max(size.width, 0)),
        height: Int32(max(size.height, 0)),
        error: error ?? ""
    )
}

enum Playback: Equatable {
    case pipeline(String)
    case buffered(String)
}

func wantedPlayback(pipeline: String?, buffered: String?) -> Playback? {
    buffered.map(Playback.buffered) ?? pipeline.map(Playback.pipeline)
}

final class BufferedPlayers: @unchecked Sendable {
    static let shared = BufferedPlayers()

    private let lock = NSLock()
    private var readings: [Int32: BufferedReading] = [:]
    private var players: [Int32: AVPlayer] = [:]

    @MainActor
    func player(_ channel: Int32) -> AVPlayer {
        if let made = players[channel] { return made }
        let made = AVPlayer()
        made.isMuted = true
        players[channel] = made
        return made
    }

    func play(_ channel: Int32, _ address: String) {
        forget(channel)
        onMain {
            let player = self.player(channel)
            player.replaceCurrentItem(with: URL(string: address).map(AVPlayerItem.init(url:)))
            player.play()
        }
    }

    func stop(_ channel: Int32) {
        forget(channel)
        onMain { self.players[channel]?.replaceCurrentItem(with: nil) }
    }

    func refresh(_ channel: Int32) {
        onMain {
            guard let player = self.players[channel] else { return }
            let item = player.currentItem
            let reading = bufferedReading(
                ready: item?.status == .readyToPlay,
                seconds: player.currentTime().seconds,
                size: item?.presentationSize ?? .zero,
                error: (item?.error ?? player.error)?.localizedDescription
            )
            self.lock.withLock { self.readings[channel] = item == nil ? nil : reading }
        }
    }

    func reading(_ channel: Int32) -> BufferedReading {
        lock.withLock { readings[channel] } ?? BufferedReading()
    }

    private func forget(_ channel: Int32) {
        lock.withLock { readings[channel] = nil }
    }
}
