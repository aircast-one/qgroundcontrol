import CoreHaptics
import SwiftUI

let SPEECH_VIEW = "view.speech"
private let LOST_BUZZ: [Int64] = [0, 400, 150, 400, 150, 400]
private let REGAINED_BUZZ: [Int64] = [0, 120]

struct SpokenLine: Equatable {
    let sequence: Int64
    let text: String
    let volume: Float
}

struct SpeechBatch: Equatable {
    let last: Int64
    let lines: [SpokenLine]
}

func speechBatch(_ view: JSON?) -> SpeechBatch? {
    guard let speech = view, speech["class"].string == "Speech" else { return nil }
    return SpeechBatch(
        last: speech["last"].int64 ?? 0,
        lines: speech["lines"].objects.map {
            SpokenLine(sequence: $0["sequence"].int64 ?? 0, text: $0["text"].string, volume: Float($0["volume"].double(1.0)))
        }
    )
}

func speechPath(_ after: Int64) -> String { "\(SPEECH_VIEW)(\(after))" }

func linkBuzz(_ lost: Bool) -> [Int64] { lost ? LOST_BUZZ : REGAINED_BUZZ }

struct ReturnAlert: Equatable {
    let speak: Bool
    let alerted: Bool
}

func returnAlert(_ alerted: Bool, _ returnNow: Bool, _ flying: Bool) -> ReturnAlert {
    ReturnAlert(speak: returnNow && !alerted, alerted: flying && (alerted || returnNow))
}

struct VoiceAlerts: View {
    var body: some View {
        LinkLossBuzz()
        ReturnHomeAlert()
    }
}

private struct ReturnHomeAlert: View {
    @QgcPath(BATTERY_VIEW) private var batteryJson
    @QgcPath(FLY_STATE) private var flyJson
    @State private var alerted = false

    var body: some View {
        let returnNow = batteryHeadline(batteryJson)?.returnNow == true
        let flying = flyJson?["flying"].bool == true
        Color.clear
            .frame(width: 0, height: 0)
            .onChange(of: [returnNow, flying], initial: true) {
                let next = returnAlert(alerted, returnNow, flying)
                alerted = next.alerted
                guard next.speak else { return }
                SpeechOut.say(RETURN_NOW_SPOKEN)
                LinkHaptics.play(linkBuzz(true), lost: true)
            }
    }
}

private struct LinkLossBuzz: View {
    @QgcPath(FLY_STATE) private var flyJson
    @State private var heard: Bool?

    var body: some View {
        let lost = flyState(flyJson)?.contactLost == true
        Color.clear
            .frame(width: 0, height: 0)
            .onChange(of: lost, initial: true) { _, now in
                if heard != nil || now { LinkHaptics.play(linkBuzz(now), lost: now) }
                heard = now
            }
    }
}

@MainActor
private enum LinkHaptics {
    static var engine: CHHapticEngine?

    static func play(_ waveform: [Int64], lost: Bool) {
        guard CHHapticEngine.capabilitiesForHardware().supportsHaptics else {
            UINotificationFeedbackGenerator().notificationOccurred(lost ? .error : .success)
            return
        }
        let events = waveform.indices.filter { $0 % 2 == 1 }.map { index in
            CHHapticEvent(
                eventType: .hapticContinuous,
                parameters: [CHHapticEventParameter(parameterID: .hapticIntensity, value: 1)],
                relativeTime: Double(waveform[..<index].reduce(0, +)) / 1000,
                duration: Double(waveform[index]) / 1000
            )
        }
        let player = (engine ?? (try? CHHapticEngine())).flatMap { running -> CHHapticPatternPlayer? in
            engine = running
            try? running.start()
            return (try? CHHapticPattern(events: events, parameters: [])).flatMap { try? running.makePlayer(with: $0) }
        }
        try? player?.start(atTime: CHHapticTimeImmediate)
    }
}
