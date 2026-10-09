import AVFoundation
import Foundation

final class SpeechOut: NSObject, AVSpeechSynthesizerDelegate, @unchecked Sendable {
    private static let pollInterval = 0.5
    private static let maxTextQueue = 20
    static let sessionOptions: AVAudioSession.CategoryOptions = [.mixWithOthers]
    private static var shared: SpeechOut?

    private let synthesizer = AVSpeechSynthesizer()
    private var timer: DispatchSourceTimer?
    private var after: Int64?
    private var waiting = 0
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
        }
        speech.synthesizer.stopSpeaking(at: .immediate)
        shared = nil
    }

    private func flush() {
        synthesizer.stopSpeaking(at: .immediate)
        waiting = 0
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
        batch.lines.filter { $0.sequence > seen }.forEach { line in
            if waiting >= SpeechOut.maxTextQueue { flush() }
            let utterance = AVSpeechUtterance(string: line.text)
            utterance.volume = line.volume
            utterance.voice = AVSpeechSynthesisVoice(language: "en-US")
            waiting += 1
            synthesizer.speak(utterance)
        }
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        queue.async { self.waiting = max(0, self.waiting - 1) }
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
        queue.async { self.waiting = max(0, self.waiting - 1) }
    }
}
