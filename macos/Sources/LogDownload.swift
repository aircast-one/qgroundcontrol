import Foundation

final class LogDownloadStore: ObservableObject, Probeable {
    static let probeID = "logDownload"

    @Published private(set) var logs: [LogEntry] = []
    @Published private(set) var requestingList = false
    @Published private(set) var downloading = false
    @Published private(set) var status = ""
    @Published private(set) var savePath = ""

    func reload() {
        let controller = Bridge.group("logDownload")
        guard controller["kind"] as? String == "object" else {
            status = "Log download is not available."
            logs = []
            return
        }

        savePath = (Bridge.group("settings.appSettings")["logSavePath"] as? String) ?? ""
        requestingList = (controller["requestingList"] as? NSNumber)?.boolValue ?? false
        downloading = (controller["downloadingLogs"] as? NSNumber)?.boolValue ?? false
        logs = LogEntry.from((Bridge.group("logDownload.model")["elements"] as? [Any]) ?? [])
        status = ""
    }

    func refresh() {
        Bridge.invoke("logDownload.refresh")
        requestingList = true
        reload()
    }

    func download(_ entry: LogEntry) {
        guard let index = logs.firstIndex(of: entry) else { return }
        _ = Bridge.set("logDownload.model.\(index).selected", true)
        Bridge.invoke("logDownload.download")
        downloading = true
        reload()
    }

    func cancel() {
        Bridge.invoke("logDownload.cancel")
        reload()
    }

    func probeState() -> [String: Any] {
        ["count": logs.count, "requestingList": requestingList, "savePath": savePath,
         "downloading": downloading, "status": status,
         "logs": logs.prefix(8).map {
             ["id": $0.id, "size": $0.sizeText, "status": $0.status, "time": $0.time]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "reload": reload()
        case "refresh": refresh()
        case "cancel": cancel()
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
