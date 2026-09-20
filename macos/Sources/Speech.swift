import AVFoundation
import Foundation
import QGCSpeechC

private let synthesizer = AVSpeechSynthesizer()

private let speechTrampoline: QGCSpeechHandler = { text, volume in
    guard let text, let spoken = String(validatingCString: text), !spoken.isEmpty else { return }
    let clamped = Float(max(0.0, min(1.0, volume)))
    DispatchQueue.main.async {
        let utterance = AVSpeechUtterance(string: spoken)
        utterance.volume = clamped
        synthesizer.speak(utterance)
    }
}

enum Speech {
    static func install() {
        qgc_set_speech_handler(speechTrampoline)
    }

    static func shutdown() {
        qgc_set_speech_handler(nil)
        synthesizer.stopSpeaking(at: .immediate)
    }
}
