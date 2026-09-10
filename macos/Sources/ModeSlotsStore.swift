import Foundation

final class ModeSlotsStore: ObservableObject, Probeable {
    static let probeID = "modeSlots"

    @Published private(set) var slots = ModeSlots.none

    private var watchPoll: Timer?

    // The transmitter switch moves while the operator is looking at this page, so this polls
    // faster than a settings page would; it is the one thing here that changes by itself.
    func startWatching() {
        guard watchPoll == nil else { return }
        refresh()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    func refresh() {
        let read = ModeSlots(Bridge.group("view.modeSlots"))
        if read != slots { slots = read }
    }

    func probeState() -> [String: Any] {
        ["available": slots.available, "channel": slots.channel, "liveSlot": slots.liveSlot,
         "reason": slots.reason, "watching": watchPoll != nil,
         "slots": slots.slots.map { ["slot": $0.slot, "mode": $0.mode ?? "", "live": $0.live] }]
    }
}
