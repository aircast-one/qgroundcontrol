import Foundation

struct ModeSlot: Equatable {
    let slot: Int
    let mode: String?
    let live: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let slot = (json["slot"] as? NSNumber)?.intValue else { return nil }
        self.slot = slot
        mode = json["mode"] as? String
        live = (json["live"] as? NSNumber)?.boolValue ?? false
    }
}

struct ModeSlots: Equatable {
    let available: Bool
    let channel: Int
    let liveSlot: Int
    let slots: [ModeSlot]

    // Why no position is lit - the transmitter is silent on the mode channel, or the switch sits
    // between thresholds. The head draws this rather than guessing at a position, because showing
    // the wrong one is worse than showing none.
    let reason: String

    static let none = ModeSlots(available: false, channel: 0, liveSlot: 0, slots: [], reason: "")

    init(available: Bool, channel: Int, liveSlot: Int, slots: [ModeSlot], reason: String) {
        self.available = available
        self.channel = channel
        self.liveSlot = liveSlot
        self.slots = slots
        self.reason = reason
    }

    init(_ json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        channel = (json["channel"] as? NSNumber)?.intValue ?? 0
        liveSlot = (json["liveSlot"] as? NSNumber)?.intValue ?? 0
        slots = ((json["slots"] as? [Any]) ?? []).compactMap(ModeSlot.init)
        reason = (json["reason"] as? String) ?? ""
    }

    // Which position the transmitter is selecting, which is not the same question as which
    // position happens to hold the mode the vehicle is flying. Two positions set to the same mode
    // both matched that older test, and a mode chosen from the ground station matched whichever
    // position held it while the switch sat somewhere else entirely.
    //
    // Requiring available as well guards a partial decode HERE, not a core that contradicts
    // itself: the core pins as an invariant that liveSlot is 0 and no slot carries live in every
    // absent state, so those two cannot disagree. Naming which one it defends against is the
    // point - an unexplained defensive check reads as superstition and gets deleted.
    func isLive(_ position: Int) -> Bool {
        available && liveSlot == position && slots.contains { $0.slot == position && $0.live }
    }

    var describes: Bool { available && !slots.isEmpty }
}
