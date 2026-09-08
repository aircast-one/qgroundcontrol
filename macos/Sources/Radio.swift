import Foundation

final class RadioStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "radio"

    @Published private(set) var state = RadioState.disconnected

    private var timer: Timer?

    func start() {
        guard timer == nil else { return }
        refresh()
        let timer = Timer(timeInterval: 0.3, repeats: true) { [weak self] _ in self?.refresh() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    func refresh() {
        let read = Radio.read(Bridge.group("radioCal"))
        if read != state { state = read }
    }

    func setTransmitterMode(_ mode: Int) {
        guard mode == 1 || mode == 2, !state.calibrating else { return }
        write("radioCal.transmitterMode", mode, "the transmitter mode")
        refresh()
    }

    func next() {
        guard state.nextEnabled else { return }
        Bridge.invoke("radioCal.nextButtonClicked")
        refresh()
    }

    func skip() {
        guard state.skipEnabled else { return }
        Bridge.invoke("radioCal.skipButtonClicked")
        refresh()
    }

    func cancel() {
        guard state.cancelEnabled else { return }
        Bridge.invoke("radioCal.cancelButtonClicked")
        refresh()
    }

    func probeState() -> [String: Any] {
        ["writeFailure": writeFailure ?? "",
         "connected": state.connected, "channelCount": state.channelCount,
         "live": state.liveChannels.count, "summary": state.summary,
         "shortfall": state.shortfall, "calibrating": state.calibrating,
         "transmitterMode": state.transmitterMode, "nextText": state.nextText,
         "statusText": state.statusText,
         "channels": state.channels.prefix(8).map(\.value),
         "sticks": state.sticks.map { ["title": $0.title, "value": $0.valueText] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
