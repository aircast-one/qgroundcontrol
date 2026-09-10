import Foundation

final class HostNoticeStore: ObservableObject, Probeable {
    static let probeID = "notices"

    @Published private(set) var queue = HostNotices.none

    private var watchPoll: Timer?

    func startWatching() {
        guard watchPoll == nil else { return }
        refresh()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    func refresh() {
        let read = HostNotices(Bridge.group("host"))
        if read != queue { queue = read }
    }

    func dismiss(_ notice: HostNotice) {
        _ = Bridge.invoke("host.acknowledge", [NSNumber(value: notice.id)])
        refresh()
    }

    // acknowledgeThrough clears everything queued at or before the newest one on screen, so a
    // notice that arrived while the operator was reading is not cleared unseen.
    func dismissAll() {
        guard let newest = queue.shown.last else { return }
        _ = Bridge.invoke("host.acknowledgeThrough", [NSNumber(value: newest.id)])
        refresh()
    }

    // Acknowledging on the press is what stops a queued request being re-offered every poll.
    func openSetup() {
        guard queue.offersSetup, let request = queue.navigation else { return }
        VehicleSetupWindow.shared.show()
        _ = Bridge.invoke("host.acknowledge", [NSNumber(value: request.id)])
        refresh()
    }

    func probeState() -> [String: Any] {
        ["count": queue.all.count, "shown": queue.shown.count, "dropped": queue.dropped,
         "watching": watchPoll != nil, "offersSetup": queue.offersSetup,
         "unreachable": queue.unreachable ?? "",
         "notices": queue.all.map { ["id": Int($0.id), "kind": "\($0.kind)",
                                     "shows": $0.shows, "line": $0.line] }]
    }
}
