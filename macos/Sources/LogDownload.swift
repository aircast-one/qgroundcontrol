import Foundation

final class LogDownloadStore: ObservableObject, Probeable {
    static let probeID = "logDownload"

    @Published private(set) var logs: [LogEntry] = []
    @Published private(set) var connected = false
    @Published private(set) var requestingList = false
    @Published private(set) var downloading = false
    @Published private(set) var status = ""
    @Published private(set) var savePath = ""
    @Published private(set) var canRefresh = false
    @Published private(set) var canDownload = false
    @Published private(set) var canCancel = false
    @Published private(set) var canErase = false
    @Published private(set) var emptyText = ""
    @Published private(set) var eraseWarning = ""
    @Published var confirmingErase = false

    func reload() {
        let view = Bridge.group("view.logs")
        guard view["kind"] as? String == "object" else {
            if status.isEmpty { status = "Log download is not available." }
            if !logs.isEmpty { logs = [] }
            if connected { connected = false }
            return
        }

        func flag(_ name: String) -> Bool { (view[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (view[name] as? String) ?? "" }

        let read = LogEntry.list(view["entries"])
        if read != logs { logs = read }
        if flag("connected") != connected { connected = flag("connected") }
        if flag("requestingList") != requestingList { requestingList = flag("requestingList") }
        if flag("downloading") != downloading { downloading = flag("downloading") }
        if flag("canRefresh") != canRefresh { canRefresh = flag("canRefresh") }
        if flag("canDownload") != canDownload { canDownload = flag("canDownload") }
        if flag("canCancel") != canCancel { canCancel = flag("canCancel") }
        if flag("canErase") != canErase { canErase = flag("canErase") }
        if text("emptyText") != emptyText { emptyText = text("emptyText") }
        if text("eraseWarning") != eraseWarning { eraseWarning = text("eraseWarning") }

        let path = (Bridge.group("settings.appSettings")["logSavePath"] as? String) ?? ""
        if path != savePath { savePath = path }
        if !status.isEmpty { status = "" }
    }

    func refresh() {
        guard canRefresh else { return }
        Bridge.invoke("logDownload.refresh")
        requestingList = true
        reload()
    }

    func download(_ entry: LogEntry) {
        guard canDownload else { return }
        _ = Bridge.set("logDownload.model.\(entry.index).selected", true)
        Bridge.invoke("logDownload.download")
        downloading = true
        reload()
    }

    func cancel() {
        guard canCancel else { return }
        Bridge.invoke("logDownload.cancel")
        reload()
    }

    func askToEraseAll() {
        guard canErase else { return }
        confirmingErase = true
    }

    func eraseAll() {
        confirmingErase = false
        Bridge.invoke("logDownload.eraseAll")
        reload()
    }

    func probeState() -> [String: Any] {
        ["count": logs.count, "requestingList": requestingList, "savePath": savePath,
         "downloading": downloading, "status": status, "canErase": canErase,
         "connected": connected, "canRefresh": canRefresh,
         "canDownload": canDownload, "canCancel": canCancel,
         "confirmingErase": confirmingErase,
         "logs": logs.prefix(8).map {
             ["id": $0.id, "size": $0.sizeText, "status": $0.status, "time": $0.time]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
        case "refresh": refresh()
        case "cancel": cancel()
        case "cancelErase":
            confirmingErase = false
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
