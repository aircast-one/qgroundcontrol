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

    // The probe's three actions all stop inside this process: post() puts a row in an in-memory
    // queue that only this window draws, dismiss() takes one out again, and openSetup() opens a
    // window. None of them can reach a vehicle, the shared settings file or a link, which is why
    // this probe is no longer read-only. It is also the only way to see this channel work: every
    // one of QGCApplication's own posters needs a vehicle, an upload or a settings write.
    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "post":
            let posted = Bridge.invoke("host.postNotice",
                                       [args["kind"] ?? "message", args["title"] ?? "",
                                        args["text"] ?? ""])["result"] as? NSNumber
            refresh()
            guard posted?.boolValue == true else {
                return ["ok": false, "error": "the core refused the kind \(args["kind"] ?? "")"]
            }
        case "dismiss":
            // Not "id": the probe's own selector is id=notices, so an action argument of that
            // name is read as the probe's and never reaches here.
            guard let wanted = Int64(args["notice"] ?? ""),
                  let notice = queue.all.first(where: { $0.id == wanted }) else {
                return ["ok": false, "error": "dismiss needs notice=<id of a queued notice>"]
            }
            dismiss(notice)
        case "openSetup":
            guard queue.offersSetup else {
                return ["ok": false, "error": "no setup request is queued"]
            }
            openSetup()
        default:
            return ["ok": false, "error": "notices has no action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }

    func probeState() -> [String: Any] {
        ["count": queue.all.count, "shown": queue.shown.count, "dropped": queue.dropped,
         "watching": watchPoll != nil, "offersSetup": queue.offersSetup,
         "reported": queue.reported, "lost": queue.lost,
         "unreachable": queue.unreachable ?? "",
         "notices": queue.all.map { ["id": Int($0.id), "kind": "\($0.kind)",
                                     "shows": $0.shows, "line": $0.line] }]
    }
}
