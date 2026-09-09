import Foundation

struct LinkConfig: Identifiable, Equatable {
    enum Editing {
        case hostAndPort
        case portOnly
        case serial
        case logFile
        case none
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "hostAndPort": self = .hostAndPort
            case "portOnly": self = .portOnly
            case "serial": self = .serial
            case "logFile": self = .logFile
            case "none": self = .none
            default: self = .unknown
            }
        }
    }

    let index: Int
    let path: String
    let name: String
    let type: String
    let typeLabel: String
    let editing: Editing
    let displaySummary: String
    let connected: Bool
    let autoConnect: Bool
    let host: String
    let port: Int
    let portName: String
    let baud: Int
    let filename: String
    let logFileName: String
    let lastError: String

    var id: Int { index }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let index = (json["index"] as? NSNumber)?.intValue,
              let type = json["type"] as? String else { return nil }
        self.index = index
        self.type = type
        editing = Editing(json["editing"] as? String)
        path = (json["path"] as? String) ?? ""
        name = (json["name"] as? String) ?? ""
        typeLabel = (json["typeLabel"] as? String) ?? ""
        displaySummary = (json["displaySummary"] as? String) ?? ""
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        autoConnect = (json["autoConnect"] as? NSNumber)?.boolValue ?? false
        host = (json["host"] as? String) ?? ""
        port = (json["port"] as? NSNumber)?.intValue ?? 0
        portName = (json["portName"] as? String) ?? ""
        baud = (json["baud"] as? NSNumber)?.intValue ?? 0
        filename = (json["filename"] as? String) ?? ""
        logFileName = (json["logFileName"] as? String) ?? ""
        lastError = (json["lastError"] as? String) ?? ""
    }

    static func list(_ json: Any?) -> [LinkConfig] {
        ((json as? [Any]) ?? []).compactMap(LinkConfig.init)
    }
}
