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
    let statusLine: String
    let connected: Bool
    let heardVehicle: Bool
    let autoConnect: Bool
    let host: String
    let port: Int?
    let portName: String
    let baud: Int?
    let filename: String
    let logFileName: String
    let lastError: String

    var id: Int { index }

    var portText: String { port.map(String.init) ?? "" }

    var baudText: String { baud.map(String.init) ?? "" }

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
        statusLine = (json["statusLine"] as? String) ?? ""
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        heardVehicle = (json["heardVehicle"] as? NSNumber)?.boolValue ?? false
        autoConnect = (json["autoConnect"] as? NSNumber)?.boolValue ?? false
        host = (json["host"] as? String) ?? ""
        port = (json["port"] as? NSNumber)?.intValue
        portName = (json["portName"] as? String) ?? ""
        baud = (json["baud"] as? NSNumber)?.intValue
        filename = (json["filename"] as? String) ?? ""
        logFileName = (json["logFileName"] as? String) ?? ""
        lastError = (json["lastError"] as? String) ?? ""
    }

    enum Health {
        case closed
        case waiting
        case heard
    }

    var health: Health {
        guard connected else { return .closed }
        return heardVehicle ? .heard : .waiting
    }

    static func list(_ json: Any?) -> [LinkConfig] {
        ((json as? [Any]) ?? []).compactMap(LinkConfig.init)
    }
}

enum LinkTypes {
    static let serial = "serial"

    static func isSerial(_ ids: [String], at index: Int) -> Bool {
        ids.indices.contains(index) && ids[index] == serial
    }
}
