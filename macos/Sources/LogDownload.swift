import Foundation

final class LogDownloadStore: ObservableObject, Probeable {
    static let probeID = "logDownload"

    @Published private(set) var logs: [LogEntry] = []
    @Published private(set) var connected = false
    @Published private(set) var requestingList = false
    @Published private(set) var downloading = false
    @Published private(set) var status = ""
    @Published private(set) var savePath = ""
    @Published var confirmingErase = false

    func reload() {
        let controller = Bridge.group("logDownload")
        guard controller["kind"] as? String == "object" else {
            status = "Log download is not available."
            logs = []
            connected = false
            return
        }

        connected = Bridge.group("vehicle")["kind"] as? String == "object"
        savePath = (Bridge.group("settings.appSettings")["logSavePath"] as? String) ?? ""
        requestingList = (controller["requestingList"] as? NSNumber)?.boolValue ?? false
        downloading = (controller["downloadingLogs"] as? NSNumber)?.boolValue ?? false
        logs = LogEntry.from((Bridge.group("logDownload.model")["elements"] as? [Any]) ?? [])
        status = ""
    }

    func refresh() {
        guard canRefresh else { return }
        Bridge.invoke("logDownload.refresh")
        requestingList = true
        reload()
    }

    func download(_ entry: LogEntry) {
        guard canDownload, let index = logs.firstIndex(of: entry) else { return }
        _ = Bridge.set("logDownload.model.\(index).selected", true)
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

    var canRefresh: Bool {
        LogDownloadRules.canRefresh(connected: connected,
                                    requestingList: requestingList, downloading: downloading)
    }

    var canDownload: Bool {
        LogDownloadRules.canDownload(requestingList: requestingList, downloading: downloading)
    }

    var canCancel: Bool {
        LogDownloadRules.canCancel(requestingList: requestingList, downloading: downloading)
    }

    var canErase: Bool {
        LogDownloadRules.canErase(count: logs.count,
                                  requestingList: requestingList, downloading: downloading)
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
        case "askToEraseAll":
            guard canErase else {
                return ["ok": false, "error": "there is nothing to erase, or the vehicle is busy"]
            }
            askToEraseAll()
        case "eraseAll":
            guard confirmingErase else {
                return ["ok": false, "error": "erasing every log needs confirming first"]
            }
            eraseAll()
        case "cancelErase":
            confirmingErase = false
        case "download":
            guard let entry = logs.first(where: { $0.id == Int(args["log"] ?? "") ?? -1 }) else {
                return ["ok": false, "error": "no log numbered \(args["log"] ?? "")"]
            }
            download(entry)
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
