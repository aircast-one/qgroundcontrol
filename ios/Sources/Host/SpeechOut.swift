import AVFoundation
import Foundation

final class SpeechOut: NSObject, AVSpeechSynthesizerDelegate, AVAudioPlayerDelegate, @unchecked Sendable {
    private static let pollInterval = 0.5
    private static let maxTextQueue = 20
    static let sessionOptions: AVAudioSession.CategoryOptions = [.mixWithOthers]
    private static var shared: SpeechOut?

    private let synthesizer = AVSpeechSynthesizer()
    private var timer: DispatchSourceTimer?
    private var after: Int64?
    private var pending: [SpokenLine] = []
    private var sounding: AnyObject?
    private var muted = false
    private let queue = DispatchQueue(label: "one.aircast.speech")

    static func start() {
        guard shared == nil else { return }
        let speech = SpeechOut()
        shared = speech
        speech.synthesizer.delegate = speech
        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .spokenAudio, options: sessionOptions)
        let timer = DispatchSource.makeTimerSource(queue: speech.queue)
        timer.schedule(deadline: .now() + pollInterval, repeating: pollInterval)
        timer.setEventHandler { [weak speech] in speech?.poll() }
        timer.resume()
        speech.timer = timer
    }

    static func stop() {
        guard let speech = shared else { return }
        speech.queue.sync {
            speech.timer?.cancel()
            speech.timer = nil
            speech.flush()
        }
        shared = nil
    }

    static func say(_ text: String, _ clip: String) {
        guard let speech = shared else { return }
        speech.queue.async { if !speech.muted { speech.enqueue(SpokenLine(sequence: 0, text: text, clip: clip, volume: 1)) } }
    }

    private func flush() {
        pending = []
        (sounding as? AVAudioPlayer)?.stop()
        sounding = nil
        synthesizer.stopSpeaking(at: .immediate)
    }

    private func poll() {
        let view = Qgc.get(after.map(speechPath) ?? SPEECH_VIEW)
        guard let batch = speechBatch(view) else { return }
        let seen = after
        after = batch.last
        let nowMuted = view["muted"].bool
        if nowMuted && !muted { flush() }
        muted = nowMuted
        guard let seen else { return }
        batch.lines.filter { $0.sequence > seen }.forEach(enqueue)
    }

    private func enqueue(_ line: SpokenLine) {
        if pending.count >= SpeechOut.maxTextQueue { flush() }
        pending = pending + [line]
        if sounding == nil { next() }
    }

    private func next() {
        guard let line = pending.first else {
            sounding = nil
            return
        }
        pending = Array(pending.dropFirst())
        guard let url = Bundle.main.url(forResource: line.clip, withExtension: "m4a", subdirectory: "raw"), let player = try? AVAudioPlayer(contentsOf: url) else {
            let utterance = AVSpeechUtterance(string: line.text)
            utterance.volume = line.volume
            utterance.voice = AVSpeechSynthesisVoice(language: "en-US")
            sounding = utterance
            synthesizer.speak(utterance)
            return
        }
        player.volume = line.volume
        player.delegate = self
        sounding = player
        if !player.play() { next() }
    }

    private func finished(_ item: AnyObject) {
        queue.async { if self.sounding === item { self.next() } }
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        finished(utterance)
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
        finished(utterance)
    }

    func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        finished(player)
    }

    func audioPlayerDecodeErrorDidOccur(_ player: AVAudioPlayer, error: Error?) {
        finished(player)
    }
}
